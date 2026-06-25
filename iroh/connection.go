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

type SendStream struct {
	ptr *C.SendStream_t
}

type RecvStream struct {
	ptr *C.RecvStream_t
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

// TODO after the option resolver is done
// func (c *Connection) CloseReason() string {
// 	res := C.connection_close_reason(c.ptr)
// 	return BytesToString(res)
// }

func (c *Connection) Close(errCode int64, reason string) error {
	bytes := ToVec(reason)

	res := C.connection_close(c.ptr, C.int64_t(errCode), bytes)

	return ResultVoid(res)
}

func (c *Connection) Datagram(data []byte) error {
	bytes := ToVec(data)

	res := C.connection_datagram(c.ptr, bytes)

	return ResultVoid(res)
}

// TODO after the option resolver is done
// func (c *Connection) MaxDatagramSize() error {
// 	res := C.connection_max_datagram_size(c.ptr)
// 	return Option(res)
// }

func (c *Connection) RemoteID() EndpointId {
	id := C.connection_remote_id(c.ptr)

	key := make([]byte, 32)

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

// TODO after the option resolver is done
// func (c *Connection) Rtt() uint64 {
// 	return uint64(C.connection_rtt(c.ptr))
// }

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
	paths := make([]PathSnapshot, v.len)

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
