package iroh

// Flat snapshot of the headline numbers from `noq::ConnectionStats`.
type ConnectionStats struct {
	UdpTxDataGrams int64
	UdpTxBytes     int64
	UdpRxDataGrams int64
	UdpRxBytes     int64
	LostPackets    int64
	LostBytes      int64
}
