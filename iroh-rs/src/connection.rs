use std::sync::Arc;

use iroh::endpoint;
use safer_ffi::{derive_ReprC, ffi_export, prelude::{c_slice, repr_c}};

use tokio::sync::Mutex;
use crate::{ConnectionStats, EndpointId, IrohError, IrohResult, PathChangeCallback, PathEventCallback, PathSnapshot, Side, iroh_executor, path, watch::WatchHandle};

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
    /// This needs to be freed
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
    pub fn remote_id(&self) -> EndpointId {
        self.0.remote_id().into()
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
        path::snapshot_paths(self.0.clone())
    }

    /// Register a callback that fires with the current set of open paths
    /// whenever the path list (or selected path) changes.
    pub fn watch_paths(&self, callback: Arc<dyn PathChangeCallback>) -> Arc<WatchHandle> {
        Arc::new(path::spawn_paths_watch(self.0.clone(), callback))
    }

    // /// Register a callback that fires for each individual path event (path
    // /// opened, closed, selected, or lagged).
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

#[ffi_export]
pub fn connection_alpn(connection: &Connection) -> repr_c::Vec<u8> {
    connection.alpn().into()
}

#[ffi_export(executor=crate::iroh_executor)]
pub async fn connection_open_uni(connection: &Connection) -> IrohResult<repr_c::Box<SendStream>> {
    ffi_await!(async{
        match connection.open_uni().await{
            Ok(ss) => {IrohResult::ok(Box::from(ss).into())},
            Err(e) => {IrohResult::err(e)}
        }
    })
}

#[ffi_export(executor=crate::iroh_executor)]
pub async fn connection_accept_uni(connection: &Connection) -> IrohResult<repr_c::Box<RecvStream>> {
    ffi_await!(async{
        match connection.accept_uni().await{
            Ok(rs) => {IrohResult::ok(Box::new(rs).into())},
            Err(e) => {IrohResult::err(e)}
        }
    })
}

#[ffi_export(executor=crate::iroh_executor)]
pub async fn connection_open_bi(connection: &Connection) -> IrohResult<repr_c::Box<BiStream>> {
    ffi_await!(async{
        match connection.open_bi().await{
            Ok(bs) => {IrohResult::ok(Box::new(bs).into())},
            Err(e) => {IrohResult::err(e)}
        }
    })
}

#[ffi_export(executor=crate::iroh_executor)]
pub async fn connection_accept_bi(connection: &Connection) -> IrohResult<repr_c::Box<BiStream>> {
    ffi_await!(async{
        match connection.accept_bi().await{
            Ok(bs) => {IrohResult::ok(Box::new(bs).into())},
            Err(e) => {IrohResult::err(e)}
        }
    })
}

#[ffi_export(executor=crate::iroh_executor)]
pub async fn connection_read_datagram(connection: &Connection) -> IrohResult<repr_c::Vec<u8>> {
    ffi_await!(async{
        match connection.read_datagram().await{
            Ok(bs) => {IrohResult::ok(bs.into())},
            Err(e) => {IrohResult::err(e)}
        }
    })
}

#[ffi_export(executor=crate::iroh_executor)]
pub async fn connection_closed(connection: &Connection) -> repr_c::String {
    ffi_await!(async{
        connection.closed().await.into()
    })
}

#[ffi_export]
pub fn connection_close_reason(connection: &Connection) -> IrohResult<repr_c::String> {
    match connection.close_reason(){
        Some(reason) => {IrohResult::ok(reason.into())},
        None => {IrohResult::err(anyhow::anyhow!("no reason was provided").into())}
    }
}

#[ffi_export]
pub fn connection_close(connection: &Connection, error_code: i64, reason: repr_c::Vec<u8>) -> IrohResult<()> {
    IrohResult::from_result(connection.close(error_code, &reason))
} 

#[ffi_export]
pub fn connection_send_datagram(connection: &Connection, data: repr_c::Vec<u8>) -> IrohResult<()> {
    IrohResult::from_result(connection.send_datagram(data.into()))
}

#[ffi_export]
pub fn connection_max_datagram_size(connection: &Connection) -> repr_c::TaggedOption<u64> {
    connection.max_datagram_size().into()
}

#[ffi_export]
pub fn connection_datagram_send_buffer_space(connection: &Connection) -> u64 {
    connection.datagram_send_buffer_space()
}

#[ffi_export]
pub fn connection_remote_id(connection: &Connection) -> EndpointId {
    connection.remote_id()
}

#[ffi_export]
pub fn connection_stable_id(connection: &Connection) -> u64 {
    connection.stable_id()
}

#[ffi_export]
pub fn connection_rtt(connection: &Connection) -> repr_c::TaggedOption<u64> {
    connection.rtt().into()
}

#[ffi_export]
pub fn connection_stats(connection: &Connection) -> ConnectionStats{
    connection.stats().into()
}

#[ffi_export(executor=crate::iroh_executor)]
pub async fn connection_send_datagram_wait(connection: &Connection, data: repr_c::Vec<u8>) -> IrohResult<()> {
    ffi_await!( async {
        IrohResult::from_result(connection.send_datagram_wait(data.into()).await)
    })
}

#[ffi_export]
pub fn connection_side(connection: &Connection) -> Side {
    connection.side()
}

#[ffi_export]
pub fn connection_paths(connection: &Connection) -> repr_c::Vec<PathSnapshot> {
    connection.paths().into()
}

//TODO think about how to implement watch handles


#[ffi_export]
pub fn connection_set_max_concurrent_uni_streams(connection: &Connection, count: u64) -> IrohResult<()> {
    IrohResult::from_result(connection.set_max_concurrent_uni_streams(count))
}

#[ffi_export]
pub fn connection_set_max_concurrent_bi_streams(connection: &Connection, count: u64) -> IrohResult<()> {
    IrohResult::from_result(connection.set_max_concurrent_bi_streams(count))
}

#[ffi_export]
pub fn connection_set_receive_window(connection: &Connection, count: u64) -> IrohResult<()> {
    IrohResult::from_result(connection.set_receive_window(count))
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

#[ffi_export]
pub fn bitstream_send(bistream : &BiStream) -> repr_c::Box<SendStream> {
    Box::new(bistream.send()).into()
}

#[ffi_export]
pub fn bitstream_recv(bistream : &BiStream)-> repr_c::Box<RecvStream> {
    Box::new(bistream.recv()).into()
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

/// Write some bytes, returning the number actually written.
#[ffi_export(executor = crate::iroh_executor)]
pub async fn write_sendstream(stream: &SendStream, buf: repr_c::Vec<u8>) -> IrohResult<u64> {
    ffi_await!(async move {
        let mut s = stream.0.lock().await;
        IrohResult::from_result(
            s.write(&buf).await.map(|written| written as u64)
        )
    })
}


/// Write all bytes, looping as needed.
#[ffi_export(executor= crate::iroh_executor)]
pub async fn write_all(stream: &SendStream, buf: repr_c::Vec<u8>) -> IrohResult<()> {
    ffi_await!(async {
        let mut s = stream.0.lock().await;

        IrohResult::from_result(
            s.write_all(&buf)
                .await
                .map(|_| ())
        )
    })
}

/// Signal that no more data will be sent on this stream.
#[ffi_export(executor = crate::iroh_executor)]
pub async fn finish(stream: &SendStream) -> IrohResult<()> {
    ffi_await!(async{
        let mut s = stream.0.lock().await;
        IrohResult::from_result(
            s.finish()
        )
    })
}

/// Abort the stream with the given error code.
#[ffi_export(executor = crate::iroh_executor)]
pub async fn reset(stream: &SendStream, error_code: u64) -> IrohResult<()> {
    ffi_await!(async {
        let error_code = match endpoint::VarInt::from_u64(error_code) {
            Ok(v) => v,
            Err(e) => return IrohResult::err(e.into()),
        };
        let mut s = stream.0.lock().await;
        IrohResult::from_result(
            s.reset(error_code)
        )
    })
}

#[ffi_export(executor = crate::iroh_executor)]
pub async fn set_priority(stream: &SendStream, p: i32) -> IrohResult<()> {
    ffi_await!(async {
        let s = stream.0.lock().await;
        IrohResult::from_result(
            s.set_priority(p)
            .map(|_| ())
        )
    })
}

#[ffi_export(executor= crate::iroh_executor)]
pub async fn priority(stream: &SendStream) -> IrohResult<i32> {
    ffi_await!(async {
        let s = stream.0.lock().await;
        IrohResult::from_result(s.priority().map(|p| p as i32))
    })
}


#[ffi_export(executor= crate::iroh_executor)]
pub async fn stopped(stream: &SendStream) -> IrohResult<repr_c::TaggedOption<u64>> {
    ffi_await!(async  {
        let s = stream.0.lock().await;
        
        IrohResult::from_result(
            s.stopped()
            .await
            .map(|r| r.map(|r| r.into_inner()).into())
        )
    })
}

#[ffi_export(executor= crate::iroh_executor)]
pub async fn send_stream_id(stream: &SendStream) -> repr_c::String {
    ffi_await!(async  {
        let r = stream.0.lock().await;
        r.id().to_string().into()
    })
}

/// The incoming half of a QUIC stream.

#[derive_ReprC]
#[repr(opaque)]
#[derive(Clone, Debug)]
pub struct RecvStream(Arc<Mutex<endpoint::RecvStream>>);

impl RecvStream {
    fn new(s: endpoint::RecvStream) -> Self {
        RecvStream(Arc::new(Mutex::new(s)))
    }
}

#[ffi_export(executor= crate::iroh_executor)]
pub async fn read_resvstream(stream: &RecvStream, size_limit: u32) -> IrohResult<repr_c::Vec<u8>> {
    ffi_await!(async  {
        let mut buf = vec![0u8; size_limit as _];
        let mut r = stream.0.lock().await;
        IrohResult::from_result(r.read(&mut buf).await.map(|res| {
            let len = res.unwrap_or(0);
            buf.truncate(len);
            buf.into()
        }))
    })
}

#[ffi_export(executor= crate::iroh_executor)]
pub async fn read_exact(stream: &RecvStream, size: u32) -> IrohResult<repr_c::Vec<u8>> {
    ffi_await!(async  {
        let mut buf = vec![0u8; size as _];
        let mut r = stream.0.lock().await;
        IrohResult::from_result(
            r.read_exact(&mut buf).await
            .map(|_| buf.into())
        )
    })
}


#[ffi_export(executor= crate::iroh_executor)]
pub async fn read_to_end(stream: &RecvStream, size_limit: u32) -> IrohResult<repr_c::Vec<u8>> {
    ffi_await!(async {
        let mut r = stream.0.lock().await;
        IrohResult::from_result(r.read_to_end(size_limit as _)
        .await.map(|buf| buf.into()))
    })
}


#[ffi_export(executor= crate::iroh_executor)]
pub async fn recv_id(stream: &RecvStream) -> repr_c::String {
    ffi_await!(async  {
        let r = stream.0.lock().await;
        r.id().to_string().into()
    })
}


#[ffi_export(executor=crate::iroh_executor)]
pub async fn bytes_read(stream: &RecvStream) -> IrohResult<u64> {
    ffi_await!(async  {
        let r = stream.0.lock().await;
        IrohResult::from_result(r.bytes_read())
    })
}

/// Stop the incoming stream with an error code
#[ffi_export(executor = crate::iroh_executor)]
pub async fn stop(
    stream: &RecvStream,
    error_code: u64,
) -> IrohResult<()> {
    ffi_await!(async  {
        let error_code = match endpoint::VarInt::from_u64(error_code) {
            Ok(v) => v,
            Err(e) => return IrohResult::err(e.into()),
        };

        let mut r = stream.0.lock().await;

        IrohResult::from_result(
            r.stop(error_code)
        )
    })
}

#[ffi_export(executor= crate::iroh_executor)]
    pub async fn received_reset(stream : &RecvStream) -> IrohResult<repr_c::TaggedOption<u64>> {
        ffi_await!(async  {
            let mut r = stream.0.lock().await;
            IrohResult::from_result(r.received_reset()
            .await
            .map(|op| op.map(|c| c.into_inner()).into()))
        })
    }