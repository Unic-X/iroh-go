package iroh

import (
	"fmt"
	"testing"
	"time"
)

func newConnectedPair(t *testing.T) (*Connection, *Connection) {
	t.Helper()

	serverBuilder := NewEndpointBuilder()
	serverBuilder.ApplyN0()
	serverBuilder.SetAlpns([][]byte{[]byte(TestALPN)})

	server, err := serverBuilder.BindEndpoint()
	if err != nil {
		t.Fatal(err)
	}

	serverConnCh := make(chan *Connection)
	serverErrCh := make(chan error, 1)

	go func() {
		incoming := server.AcceptNext()
		if incoming == nil {
			serverErrCh <- fmt.Errorf("incoming nil")
			return
		}

		accepting, err := incoming.Accept()
		if err != nil {
			serverErrCh <- err
			return
		}

		conn, err := accepting.Connect()
		if err != nil {
			serverErrCh <- err
			return
		}

		serverConnCh <- conn
	}()

	clientBuilder := NewEndpointBuilder()
	clientBuilder.ApplyN0()
	clientBuilder.SetAlpns([][]byte{[]byte(TestALPN)})

	client, err := clientBuilder.BindEndpoint()
	if err != nil {
		t.Fatal(err)
	}

	clientConn, err := client.Connect(server.Addr(), []byte(TestALPN))
	if err != nil {
		t.Fatal(err)
	}

	var serverConn *Connection

	select {
	case serverConn = <-serverConnCh:
	case err := <-serverErrCh:
		t.Fatal(err)
	case <-time.After(5 * time.Second):
		t.Fatal("timeout waiting for server connection")
	}

	return clientConn, serverConn
}

func TestConnectionStableID(t *testing.T) {
	client, server := newConnectedPair(t)

	if client.StableId() == 0 {
		t.Fatal("client stable id is zero")
	}

	if server.StableId() == 0 {
		t.Fatal("server stable id is zero")
	}
}

func TestConnectionRTT(t *testing.T) {
	client, _ := newConnectedPair(t)

	rtt, err := client.Rtt()
	if err != nil {
		t.Fatal(err)
	}

	if rtt == nil {
		t.Skip("RTT not yet available")
	}

	t.Logf("RTT = %d", *rtt)
}

func TestConnectionMaxDatagramSize(t *testing.T) {
	client, _ := newConnectedPair(t)

	size, err := client.MaxDatagramSize()
	if err != nil {
		t.Fatal(err)
	}

	if size != nil {
		t.Logf("max datagram size = %d", *size)
	}
}

func TestConnectionAlpn(t *testing.T) {
	client, _ := newConnectedPair(t)

	alpn := string(client.Alpn())

	if alpn != TestALPN {
		t.Fatalf("expected %q got %q", TestALPN, alpn)
	}
}

func TestOpenUni(t *testing.T) {
	client, server := newConnectedPair(t)

	send, err := client.OpenUni()
	if err != nil {
		t.Fatal(err)
	}

	recv, err := server.AcceptUni()
	if err != nil {
		t.Fatal(err)
	}

	if send == nil || recv == nil {
		t.Fatal("stream is nil")
	}
}

func TestOpenBi(t *testing.T) {
	client, server := newConnectedPair(t)

	bi, err := client.OpenBi()
	if err != nil {
		t.Fatal(err)
	}

	peer, err := server.AcceptBi()
	if err != nil {
		t.Fatal(err)
	}

	if bi == nil || peer == nil {
		t.Fatal("bistream nil")
	}
}

func TestConnectionSendDatagram(t *testing.T) {
	client, server := newConnectedPair(t)

	err := client.SendDatagram([]byte("hello"))
	if err != nil {
		t.Fatal(err)
	}

	msg, err := server.ReadDatagram()
	if err != nil {
		t.Fatal(err)
	}

	if string(*msg) != "hello" {
		t.Fatalf("expected hello got %q", string(*msg))
	}
}

func TestConnectionClose(t *testing.T) {
	client, _ := newConnectedPair(t)

	err := client.Close(1, "closing")
	if err != nil {
		t.Fatal(err)
	}
}

func TestConnectionStats(t *testing.T) {
	client, _ := newConnectedPair(t)

	stats := client.Stats()

	t.Logf("%+v", stats)
}

func TestConnectionPaths(t *testing.T) {
	client, _ := newConnectedPair(t)

	paths := client.Paths()

	if len(paths) == 0 {
		t.Fatal("expected at least one path")
	}
}
