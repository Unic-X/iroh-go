package iroh

/*
#include <stdlib.h>
#include <stddef.h>
#cgo linux LDFLAGS: -L${SRCDIR}/../../iroh-rs/target/release -lgo_iroh
#include "../iroh-rs/include/iroh.h"
*/
import "C"

type PathSnapshot struct {
	Id         string
	IsSelected bool
	RemoteAddr string
	IsIp       bool
	IsRelay    bool
	RTTms      uint64
	Stats      PathStatsRecord
}

type PathStatsRecord struct {
	/// RTT estimate (ms).
	RTTms uint64 // This seems dupilcated as this also exists in PathSnapShot
	/// UDP datagrams sent on this path.
	UDPTxDatagrams uint64
	/// UDP bytes sent on this path.
	UDPTxBytes uint64
	/// UDP datagrams received on this path.
	UDPRxDatagrams uint64
	/// UDP bytes received on this path.
	UDPRxBytes uint64
	/// Current congestion window.
	Cwnd uint64
	/// Congestion events on this path.
	CongestionEvents uint64
	/// Packets considered lost on this path.
	LostPackets uint64
	/// Bytes considered lost on this path.
	LostBytes uint64
	/// Largest UDP payload this path currently supports.
	CurrentMtu uint32
}
