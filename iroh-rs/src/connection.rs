use std::sync::Arc;

use iroh::endpoint;
use safer_ffi::derive_ReprC;
use std::sync::Mutex;
use crate::{ConnectionStats, EndpointId, IrohError, Side,PathSnapshot,path};

#[derive_ReprC]
#[repr(opaque)]
pub struct Connection(pub endpoint::Connection);

impl From<endpoint::Connection> for Connection {
    fn from(value: endpoint::Connection) -> Self {
        Self(value)
    }
}



impl Connection {
    /// The ALPN protocol negotiated for this connection.
    pub fn alpn(&self) -> Vec<u8> {
        self.0.alpn().to_vec()
    }

    /// Open a new unidirectional outgoing stream.
    pub async fn open_uni(&self) -> Result<SendStream, IrohError> {
        let s = self.0.open_uni().await?;
        Ok(SendStream::new(s))
    }

    /// Accept the next incoming unidirectional stream.
    pub async fn accept_uni(&self) -> Result<RecvStream, IrohError> {
        let r = self.0.accept_uni().await?;
        Ok(RecvStream::new(r))
    }

    /// Open a new bidirectional outgoing stream.
    pub async fn open_bi(&self) -> Result<BiStream, IrohError> {
        let (s, r) = self.0.open_bi().await?;
        Ok(BiStream {
            send: SendStream::new(s),
            recv: RecvStream::new(r),
        })
    }

    /// Accept the next incoming bidirectional stream.
    pub async fn accept_bi(&self) -> Result<BiStream, IrohError> {
        let (s, r) = self.0.accept_bi().await?;
        Ok(BiStream {
            send: SendStream::new(s),
            recv: RecvStream::new(r),
        })
    }

    /// Read the next datagram from the connection.
    pub async fn read_datagram(&self) -> Result<Vec<u8>, IrohError> {
        let res = self.0.read_datagram().await?;
        Ok(res.to_vec())
    }

    /// Wait for the connection to be closed, returning the cause.
    pub async fn closed(&self) -> String {
        let err = self.0.closed().await;
        err.to_string()
    }

    /// If the connection is closed, the reason why. None if still open.
    pub fn close_reason(&self) -> Option<String> {
        self.0.close_reason().map(|s| s.to_string())
    }

    /// Close the connection immediately with the given application error code.
    ///
    /// Signed for Kotlin/Swift ergonomics; negative values are rejected.
    pub fn close(&self, error_code: i64, reason: &[u8]) -> Result<(), IrohError> {
        let unsigned =
            u64::try_from(error_code).map_err(|_| anyhow::anyhow!("error_code must be >= 0"))?;
        let code = endpoint::VarInt::from_u64(unsigned)?;
        self.0.close(code, reason);
        Ok(())
    }

    /// Send a datagram on this connection.
    pub fn send_datagram(&self, data: Vec<u8>) -> Result<(), IrohError> {
        self.0.send_datagram(data.into())?;
        Ok(())
    }

    /// Maximum size of a datagram that can currently be sent.
    pub fn max_datagram_size(&self) -> Option<u64> {
        self.0.max_datagram_size().map(|s| s as _)
    }

    /// Bytes available in the datagram send buffer.
    pub fn datagram_send_buffer_space(&self) -> u64 {
        self.0.datagram_send_buffer_space() as _
    }

    /// The [`EndpointId`] of the remote peer.
    pub fn remote_id(&self) -> Arc<EndpointId> {
        Arc::new(self.0.remote_id().into())
    }

    /// A stable identifier for this connection.
    pub fn stable_id(&self) -> u64 {
        self.0.stable_id() as _
    }

    /// Current best estimate of this connection's RTT on the selected path,
    /// in milliseconds. `None` if no path is currently selected.
    pub fn rtt(&self) -> Option<u64> {
        self.0
            .paths()
            .iter()
            .find(|p| p.is_selected())
            .map(|p| p.rtt().as_millis() as u64)
    }

    /// A flat snapshot of the most useful headline statistics for this connection.
    pub fn stats(&self) -> ConnectionStats {
        let s = self.0.stats();
        ConnectionStats {
            udp_tx_datagrams: s.udp_tx.datagrams as i64,
            udp_tx_bytes: s.udp_tx.bytes as i64,
            udp_rx_datagrams: s.udp_rx.datagrams as i64,
            udp_rx_bytes: s.udp_rx.bytes as i64,
            lost_packets: s.lost_packets as i64,
            lost_bytes: s.lost_bytes as i64,
        }
    }

    /// Like [`Connection::send_datagram`] but waits for capacity if the send
    /// buffer is full.
    pub async fn send_datagram_wait(&self, data: Vec<u8>) -> Result<(), IrohError> {
        self.0.send_datagram_wait(data.into()).await?;
        Ok(())
    }

    /// Which side of the connection we are (client or server).
    pub fn side(&self) -> Side {
        self.0.side().into()
    }

    /// A snapshot of all currently open network paths for this connection.
    pub fn paths(&self) -> Vec<PathSnapshot> {
        path::snapshot_paths(self.0)
    }

    /// Register a callback that fires with the current set of open paths
    /// whenever the path list (or selected path) changes.
    pub fn watch_paths(&self, callback: Arc<dyn PathChangeCallback>) -> Arc<WatchHandle> {
        Arc::new(path::spawn_paths_watch(self.0.clone(), callback))
    }

    /// Register a callback that fires for each individual path event (path
    /// opened, closed, selected, or lagged).
    pub fn watch_path_events(&self, callback: Arc<dyn PathEventCallback>) -> Arc<WatchHandle> {
        Arc::new(path::spawn_path_events_watch(self.0.clone(), callback))
    }

    /// Set the maximum number of concurrent incoming unidirectional streams.
    pub fn set_max_concurrent_uni_streams(&self, count: u64) -> Result<(), IrohError> {
        let n = endpoint::VarInt::from_u64(count)?;
        self.0.set_max_concurrent_uni_streams(n);
        Ok(())
    }

    /// Set the receive window for this connection.
    pub fn set_receive_window(&self, count: u64) -> Result<(), IrohError> {
        let n = endpoint::VarInt::from_u64(count)?;
        self.0.set_receive_window(n);
        Ok(())
    }

    /// Set the maximum number of concurrent incoming bidirectional streams.
    pub fn set_max_concurrent_bi_streams(&self, count: u64) -> Result<(), IrohError> {
        let n = endpoint::VarInt::from_u64(count)?;
        self.0.set_max_concurrent_bi_streams(n);
        Ok(())
    }
}


/// A bidirectional QUIC stream pair.
#[derive_ReprC]
#[repr(opaque)]
pub struct BiStream {
    send: SendStream,
    recv: RecvStream,
}


impl BiStream {
    pub fn send(&self) -> SendStream {
        self.send.clone()
    }

    pub fn recv(&self) -> RecvStream {
        self.recv.clone()
    }
}

/// The outgoing half of a QUIC stream.
#[derive_ReprC]
#[repr(opaque)]
#[derive(Debug, Clone)]
pub struct SendStream(Arc<Mutex<endpoint::SendStream>>);

impl SendStream {
    fn new(s: endpoint::SendStream) -> Self {
        SendStream(Arc::new(Mutex::new(s)))
    }
}



impl SendStream {
    /// Write some bytes, returning the number actually written.
  
    pub async fn write(&self, buf: &[u8]) -> Result<u64, IrohError> {
        let mut s = self.0.lock().await;
        let written = s.write(buf).await?;
        Ok(written as _)
    }

    /// Write all bytes, looping as needed.
    #[uniffi::method(async_runtime = "tokio")]
    pub async fn write_all(&self, buf: &[u8]) -> Result<(), IrohError> {
        let mut s = self.0.lock().await;
        s.write_all(buf).await?;
        Ok(())
    }

    /// Signal that no more data will be sent on this stream.
    #[uniffi::method(async_runtime = "tokio")]
    pub async fn finish(&self) -> Result<(), IrohError> {
        let mut s = self.0.lock().await;
        s.finish()?;
        Ok(())
    }

    /// Abort the stream with the given error code.
    #[uniffi::method(async_runtime = "tokio")]
    pub async fn reset(&self, error_code: u64) -> Result<(), IrohError> {
        let error_code = endpoint::VarInt::from_u64(error_code)?;
        let mut s = self.0.lock().await;
        s.reset(error_code)?;
        Ok(())
    }

    #[uniffi::method(async_runtime = "tokio")]
    pub async fn set_priority(&self, p: i32) -> Result<(), IrohError> {
        let s = self.0.lock().await;
        s.set_priority(p)?;
        Ok(())
    }

    #[uniffi::method(async_runtime = "tokio")]
    pub async fn priority(&self) -> Result<i32, IrohError> {
        let s = self.0.lock().await;
        let p = s.priority()?;
        Ok(p)
    }

    #[uniffi::method(async_runtime = "tokio")]
    pub async fn stopped(&self) -> Result<Option<u64>, IrohError> {
        let s = self.0.lock().await;
        let res = s.stopped().await?;
        Ok(res.map(|r| r.into_inner()))
    }

    #[uniffi::method(async_runtime = "tokio")]
    pub async fn id(&self) -> String {
        let r = self.0.lock().await;
        r.id().to_string()
    }
}

/// The incoming half of a QUIC stream.
#[derive(Clone, uniffi::Object)]
pub struct RecvStream(Arc<Mutex<endpoint::RecvStream>>);

impl RecvStream {
    fn new(s: endpoint::RecvStream) -> Self {
        RecvStream(Arc::new(Mutex::new(s)))
    }
}

#[uniffi::export]
impl RecvStream {
    /// Read up to `size_limit` bytes into a fresh buffer.
    #[uniffi::method(async_runtime = "tokio")]
    pub async fn read(&self, size_limit: u32) -> Result<Vec<u8>, IrohError> {
        let mut buf = vec![0u8; size_limit as _];
        let mut r = self.0.lock().await;
        let res = r.read(&mut buf).await?;
        let len = res.unwrap_or(0);
        buf.truncate(len);
        Ok(buf)
    }

    /// Read exactly `size` bytes, erroring if the stream ends early.
    #[uniffi::method(async_runtime = "tokio")]
    pub async fn read_exact(&self, size: u32) -> Result<Vec<u8>, IrohError> {
        let mut buf = vec![0u8; size as _];
        let mut r = self.0.lock().await;
        r.read_exact(&mut buf).await?;
        Ok(buf)
    }

    /// Read until end-of-stream, with `size_limit` as a maximum.
    #[uniffi::method(async_runtime = "tokio")]
    pub async fn read_to_end(&self, size_limit: u32) -> Result<Vec<u8>, IrohError> {
        let mut r = self.0.lock().await;
        let res = r.read_to_end(size_limit as _).await?;
        Ok(res)
    }

    #[uniffi::method(async_runtime = "tokio")]
    pub async fn id(&self) -> String {
        let r = self.0.lock().await;
        r.id().to_string()
    }

    /// Total bytes read from this stream so far.
    #[uniffi::method(async_runtime = "tokio")]
    pub async fn bytes_read(&self) -> Result<u64, IrohError> {
        let r = self.0.lock().await;
        Ok(r.bytes_read()?)
    }

    /// Stop the incoming stream with an error code.
    #[uniffi::method(async_runtime = "tokio")]
    pub async fn stop(&self, error_code: u64) -> Result<(), IrohError> {
        let error_code = endpoint::VarInt::from_u64(error_code)?;
        let mut r = self.0.lock().await;
        r.stop(error_code)?;
        Ok(())
    }

    #[uniffi::method(async_runtime = "tokio")]
    pub async fn received_reset(&self) -> Result<Option<u64>, IrohError> {
        let mut r = self.0.lock().await;
        let code = r.received_reset().await?;
        Ok(code.map(|c| c.into_inner()))
    }
}