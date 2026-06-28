package iroh

import (
	"fmt"
	"testing"
	"time"
)

func TestCustomPreset(t *testing.T) {

	// Build a custom builder

	customBuilder := NewEndpointBuilder()
	if customBuilder == nil {
		t.Errorf("expected non-nil got newendpointbuilder returned nil")
	}

	customBuilder.ApplyMinimal()
	byteSlice := [][]byte{[]byte("custom/preset/1")}
	customBuilder.SetAlpns(byteSlice)

	ep, err := customBuilder.BindEndpoint()

	if err != nil {
		t.Errorf("custom builder binding failed %s", err.Error())
	}

	if len(ep.BoundSockets()) == 0 {
		t.Errorf("no bound sockets found for custom builder binded endpoint")
	}

	ep.Close()
}

func TestSidePathsCompile(t *testing.T) {
	builder := NewEndpointBuilder()
	if builder == nil {
		t.Fatal("expected non-nil endpoint builder")
	}

	builder.ApplyMinimal()

	ep, err := builder.BindEndpoint()
	if err != nil {
		t.Fatalf("failed to bind endpoint: %v", err)
	}
	defer ep.Close()

	// Surface-level smoke test:
	// Ensure AcceptNext is callable without requiring an incoming connection.
	done := make(chan struct{})

	go func() {
		defer close(done)
		_ = ep.AcceptNext()
	}()

	select {
	case <-done:
		// AcceptNext returned immediately (e.g. endpoint closed).
	case <-time.After(10 * time.Millisecond):
		// Timeout is expected; we only care that the call compiles and blocks.
	}
}

const TestALPN = "iroh-ffi/test/0"

func TestConnectEchoRoundtrip(t *testing.T) {
	// Server
	serverBuilder := NewEndpointBuilder()
	serverBuilder.ApplyN0()
	serverBuilder.SetAlpns([][]byte{[]byte(TestALPN)})

	server, err := serverBuilder.BindEndpoint()
	if err != nil {
		t.Fatalf("failed to bind server: %v", err)
	}
	defer server.Close()

	serverAddr := server.Addr()
	serverID := server.Id().key

	serverDone := make(chan error, 1)

	go func() {
		incoming := server.AcceptNext()
		if incoming == nil {
			serverDone <- fmt.Errorf("incoming is nil")
			return
		}

		accepting, err := incoming.Accept()
		if err != nil {
			serverDone <- err
			return
		}

		conn, err := accepting.Connect()
		if err != nil {
			serverDone <- err
			return
		}

		if conn.Side() != ServerSide {
			serverDone <- fmt.Errorf("expected server side")
			return
		}

		if string(conn.Alpn()) != TestALPN {
			serverDone <- fmt.Errorf("unexpected ALPN")
			return
		}

		bi, err := conn.AcceptBi()
		if err != nil {
			serverDone <- err
			return
		}

		r := bi.Recv()
		msg, err := r.ReadToEnd(64)
		if err != nil {
			serverDone <- err
			return
		}

		s := bi.Send()

		if err := s.WriteAll(*msg); err != nil {
			serverDone <- err
			return
		}

		if err := s.Finish(); err != nil {
			serverDone <- err
			return
		}

		dg, err := conn.ReadDatagram()
		if err != nil {
			serverDone <- err
			return
		}

		if err := conn.SendDatagram(*dg); err != nil {
			serverDone <- err
			return
		}

		serverDone <- nil
	}()

	// Client
	clientBuilder := NewEndpointBuilder()
	clientBuilder.ApplyN0()

	client, err := clientBuilder.BindEndpoint()
	if err != nil {
		t.Fatalf("failed to bind client: %v", err)
	}
	defer client.Close()

	conn, err := client.Connect(serverAddr, []byte(TestALPN))
	if err != nil {
		t.Fatalf("connect failed: %v", err)
	}

	if conn.Side() != ClientSide {
		t.Fatalf("expected client side")
	}

	remoteId := conn.RemoteID().key

	if string(remoteId) != string(serverID) {
		t.Fatalf("unexpected remote ID %v, %v", remoteId, serverID)
	}

	if len(conn.Paths()) == 0 {
		t.Fatalf("expected at least one path")
	}

	bi, err := conn.OpenBi()
	if err != nil {
		t.Fatal(err)
	}

	s := bi.Send()
	if err := s.WriteAll([]byte("hello iroh")); err != nil {
		t.Fatal(err)
	}

	if err := s.Finish(); err != nil {
		t.Fatal(err)
	}

	r := bi.Recv()
	echo, err := r.ReadToEnd(64)
	if err != nil {
		t.Fatal(err)
	}

	if string(*echo) != "hello iroh" {
		t.Fatalf("expected echo, got %q", echo)
	}

	if err := conn.SendDatagram([]byte("ping")); err != nil {
		t.Fatal(err)
	}

	pong, err := conn.ReadDatagram()
	if err != nil {
		t.Fatal(err)
	}

	if string(*pong) != "ping" {
		t.Fatalf("expected ping, got %q", pong)
	}

	stats := conn.Stats()
	if stats.UdpTxDataGrams == 0 {
		t.Fatalf("expected UDP datagrams to be sent")
	}

	if err := conn.Close(0, "bye"); err != nil {
		t.Fatal(err)
	}

	if err := <-serverDone; err != nil {
		t.Fatal(err)
	}
}
