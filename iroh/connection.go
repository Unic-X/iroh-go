package iroh

/*
#include <stdlib.h>
#include <stddef.h>
#cgo linux LDFLAGS: -L${SRCDIR}/../../iroh-rs/target/release -lgo_iroh
#include "../iroh-rs/include/iroh.h"
*/
import "C"

type Connection struct {
	ptr *C.Connection_t
}

type BiStream struct {
	ptr *C.BiStream_t
}

func (b *BiStream) Send() SendStream {
	return SendStream{
		ptr: C.bitstream_send(b.ptr),
	}
}

func (b *BiStream) Recv() RecvStream {
	return RecvStream{
		ptr: C.bitstream_recv(b.ptr),
	}
}

type SendStream struct {
	ptr *C.SendStream_t
}

func (s *SendStream) WriteAll(buf []byte) error {
	res := C.write_all(s.ptr, ToVec(buf))

	return ResultVoid(res)
}

func (s *SendStream) Finish() error {
	res := C.finish(s.ptr)

	return ResultVoid(res)
}

// TODO add missing methods

type RecvStream struct {
	ptr *C.RecvStream_t
}

func (r *RecvStream) ReadToEnd(limit uint32) (*[]byte, error) {
	res := C.read_to_end(r.ptr, C.uint32_t(limit))

	return ResultValue(
		BytesToGo[[]byte](res.value._1),
		res.error,
	)
}

func (c *Connection) Alpn() []byte {
	return BytesToGo[[]byte](C.connection_alpn(c.ptr))
}

func (c *Connection) OpenUni() (*SendStream, error) {
	res := C.connection_open_uni(c.ptr)
	return ResultValue(
		SendStream{
			ptr: res.value._1,
		},
		res.error,
	)
}

func (c *Connection) AcceptUni() (*RecvStream, error) {
	res := C.connection_accept_uni(c.ptr)
	return ResultValue(
		RecvStream{
			ptr: res.value._1,
		},
		res.error,
	)
}

func (c *Connection) OpenBi() (*BiStream, error) {
	res := C.connection_open_bi(c.ptr)
	return ResultValue(
		BiStream{
			ptr: res.value._1,
		},
		res.error,
	)
}

func (c *Connection) AcceptBi() (*BiStream, error) {
	res := C.connection_accept_bi(c.ptr)
	return ResultValue(
		BiStream{
			ptr: res.value._1,
		},
		res.error,
	)
}

// Will this hack of BytesToGo work: Not sure
func (c *Connection) ReadDatagram() (*[]byte, error) {
	res := C.connection_read_datagram(c.ptr)
	if res.tag == C.IROH_RESULT_TAG_ERROR {
		return nil, ErrorFromC(res.error._1)
	}

	bytes := BytesToGo[[]byte](res.value._1)
	return &bytes, nil
}

func (c *Connection) Closed() string {
	res := C.connection_closed(c.ptr)
	return BytesToGo[string](res)
}

func (c *Connection) CloseReason() (*string, error) {
	res := C.connection_close_reason(c.ptr)

	return ResultValue(
		BytesToGo[string](res.value._1),
		res.error,
	)
}

func (c *Connection) Close(errCode int64, reason string) error {
	bytes := ToVec(reason)

	res := C.connection_close(c.ptr, C.int64_t(errCode), bytes)

	return ResultVoid(res)
}

func (c *Connection) SendDatagram(data []byte) error {
	bytes := ToVec(data)

	res := C.connection_send_datagram(c.ptr, bytes)

	return ResultVoid(res)
}

func (c *Connection) MaxDatagramSize() (*uint64, error) {
	res := C.connection_max_datagram_size(c.ptr)

	return OptionValue(
		bool(res._0),
		uint64(res._1),
	), nil
}

func (c *Connection) DatagramSendBufferSpace() uint64 {
	res := C.connection_datagram_send_buffer_space(c.ptr)

	return uint64(res)
}

func (c *Connection) RemoteID() EndpointId {
	id := C.connection_remote_id(c.ptr)

	key := make([]byte, 0, 32)

	for _, item := range id.key.idx {
		key = append(key, byte(item))
	}

	return EndpointId{
		key: key,
	}
}

func (c *Connection) StableId() uint64 {
	return uint64(C.connection_stable_id(c.ptr))
}

func (c *Connection) Rtt() (*uint64, error) {
	res := (C.connection_rtt(c.ptr))

	return OptionValue(
		bool(res._0),
		uint64(res._1),
	), nil
}

func (c *Connection) Stats() ConnectionStats {
	stats := C.connection_stats(c.ptr)

	return ConnectionStats{
		UdpTxDataGrams: int64(stats.udp_tx_datagrams),
		UdpTxBytes:     int64(stats.udp_tx_bytes),
		UdpRxDataGrams: int64(stats.udp_rx_datagrams),
		UdpRxBytes:     int64(stats.udp_rx_bytes),
		LostPackets:    int64(stats.lost_packets),
		LostBytes:      int64(stats.lost_bytes),
	}
}

func (c *Connection) SendDatagramWait(data []byte) error {
	bytes := ToVec(data)
	res := C.connection_send_datagram_wait(c.ptr, bytes)
	return ResultVoid(res)
}

func (c *Connection) Side() Side {
	return Side(C.connection_side(c.ptr))
}

func (c *Connection) Paths() []PathSnapshot {
	v := C.connection_paths(c.ptr)
	paths := make([]PathSnapshot, 0, v.len)

	for _, path := range SliceFromC(v.ptr, v.len) {
		paths = append(paths, PathSnapshot{
			Id:         BytesToGo[string](path.id),
			IsSelected: bool(path.is_selected),
			RemoteAddr: BytesToGo[string](path.remote_addr),
			IsIp:       bool(path.is_ip),
			IsRelay:    bool(path.is_relay),
			RTTms:      uint64(path.rtt_ms),
			Stats: PathStatsRecord{
				RTTms:            uint64(path.stats.rtt_ms),
				UDPTxDatagrams:   uint64(path.stats.udp_tx_datagrams),
				UDPTxBytes:       uint64(path.stats.udp_tx_bytes),
				UDPRxDatagrams:   uint64(path.stats.udp_rx_datagrams),
				UDPRxBytes:       uint64(path.stats.udp_rx_bytes),
				Cwnd:             uint64(path.stats.cwnd),
				CongestionEvents: uint64(path.stats.congestion_events),
				LostPackets:      uint64(path.stats.lost_packets),
				LostBytes:        uint64(path.stats.lost_bytes),
				CurrentMtu:       uint32(path.stats.current_mtu),
			},
		})
	}
	return paths
}

func (c *Connection) SetMaxConcurrentUniStreams(count uint64) error {
	res := C.connection_set_max_concurrent_uni_streams(c.ptr, C.uint64_t(count))
	return ResultVoid(res)
}

func (c *Connection) SetMaxConcurrentBiStreams(count uint64) error {
	res := C.connection_set_max_concurrent_bi_streams(c.ptr, C.uint64_t(count))
	return ResultVoid(res)
}

func (c *Connection) SetReceiveWindow(count uint64) error {
	res := C.connection_set_receive_window(c.ptr, C.uint64_t(count))
	return ResultVoid(res)
}

func (s *SendStream) Reset(err uint64) error {
	res := C.reset(s.ptr, C.uint64_t(err))
	return ResultVoid(res)
}

func (s *SendStream) SetPriority(p int32) error {
	res := C.set_priority(s.ptr, C.int32_t(p))

	return ResultVoid(res)
}

func (s *SendStream) Stopped() (*uint64, error) {
	res := C.stopped(s.ptr)

	return OptionValue(
		bool(res.value._0),
		uint64(*res.error._1.message.ptr),
	), nil

}

func (s *SendStream) StreamId() string {
	res := C.send_stream_id(s.ptr)

	return BytesToGo[string](res)
}

func (r *RecvStream) ReadResvStream(size uint32) (*[]byte, error) {
	res := C.read_resvstream(r.ptr, C.uint32_t(size))

	return ResultValue(
		BytesToGo[[]byte](res.value._1),
		res.error,
	)
}

func (r *RecvStream) ReadExact(size uint32) (*[]byte, error) {
	res := C.read_exact(r.ptr, C.uint32_t(size))

	return ResultValue(
		BytesToGo[[]byte](res.value._1),
		res.error,
	)
}

func (r *RecvStream) RecvId() string {
	res := C.recv_id(r.ptr)

	return BytesToGo[string](res)
}

func (r *RecvStream) BytesRead() (*uint64, error) {
	res := C.bytes_read(r.ptr)

	return ResultValue(
		uint64(res.value._1),
		res.error,
	)
}

func (r *RecvStream) Stop(err uint64) error {
	res := C.stop(r.ptr, C.uint64_t(err))

	return ResultVoid(res)

}

func (r *RecvStream) Received_reset() (*uint64, error) {
	res := C.received_reset(r.ptr)

	return OptionValue(
		bool(res.value._0),
		uint64(*res.error._1.message.ptr),
	), nil

}
