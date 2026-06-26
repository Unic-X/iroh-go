//! Raw accept loop types: [`Incoming`], [`Accepting`], [`Connecting`], and
//! related address enums.
//!
//! The router (`Endpoint::bind` with `EndpointOptions.protocols`) covers the
//! common case of "dispatch by ALPN", but exposing these types lets the FFI
//! caller run its own accept loop, inspect ALPN before completing the
//! handshake, refuse connections, etc.

use std::sync::Arc;

use safer_ffi::{derive_ReprC, ffi_export, prelude::repr_c};
use tokio::sync::Mutex;

use crate::{Connection, EndpointId, IncomingLocalAddrKind::{Custom, Ip, Relay}, IrohError, IrohResult, iroh_executor};

/// Which side of a connection we are.
#[derive_ReprC]
#[repr(u8)]
pub enum Side {
    /// We initiated this connection.
    Client,
    /// We accepted this connection.
    Server,
}

impl From<iroh::endpoint::Side> for Side {
    fn from(s: iroh::endpoint::Side) -> Self {
        match s {
            iroh::endpoint::Side::Client => Side::Client,
            iroh::endpoint::Side::Server => Side::Server,
        }
    }
}


#[derive_ReprC]
#[repr(opaque)]
#[derive(Debug, Clone)]
pub enum IncomingAddr {
    /// A direct connection from an IP address (`ip:port` string).
    Ip { addr: String },
    /// A connection via a relay.
    Relay {
        url: String,
        endpoint_id: Arc<EndpointId>,
    },
    /// A custom-transport connection (rendered as its debug form).
    Custom { description: String },
}

impl From<iroh::endpoint::IncomingAddr> for IncomingAddr {
    fn from(addr: iroh::endpoint::IncomingAddr) -> Self {
        match addr {
            iroh::endpoint::IncomingAddr::Ip(socket) => IncomingAddr::Ip {
                addr: socket.to_string(),
            },
            iroh::endpoint::IncomingAddr::Relay { url, endpoint_id } => IncomingAddr::Relay {
                url: url.to_string(),
                endpoint_id: Arc::new(endpoint_id.into()),
            },
            iroh::endpoint::IncomingAddr::Custom(custom) => IncomingAddr::Custom {
                description: format!("{custom:?}"),
            },
            _ => IncomingAddr::Custom {
                description: "unknown".into(),
            },
        }
    }
}

#[derive_ReprC]
#[repr(u8)]
#[derive(Debug, Clone, Default)]
pub enum IncomingLocalAddrKind {
    Ip,

    #[default]
    Relay,
    Custom,
}

/// The local address that received an incoming connection.
#[derive_ReprC]
#[repr(C)]
#[derive(Debug, Clone, Default)]
pub struct IncomingLocalAddr {
    kind: IncomingLocalAddrKind,
    
    /// Direct IP (`ip` string if available).
    addr: Option<repr_c::String>, // Problem
    /// Relay path.
    url: Option<repr_c::String>,
    /// Custom transport.
    description: Option<repr_c::String> ,
}

impl From<iroh::endpoint::LocalTransportAddr> for IncomingLocalAddr {
    fn from(value: iroh::endpoint::LocalTransportAddr) -> Self {
        match value {
            iroh::endpoint::LocalTransportAddr::Ip(ip) => IncomingLocalAddr {
                kind: Ip,
                addr: ip.map(|i| i.to_string().into()).into(),
                ..Default::default()
            },
            iroh::endpoint::LocalTransportAddr::Relay(url) => IncomingLocalAddr {
                kind: Relay,
                url: Some(url.to_string().into()),
                ..Default::default()

            },
            iroh::endpoint::LocalTransportAddr::Custom(custom) => IncomingLocalAddr {
                kind: Custom,
                description: custom.map(|c| format!("{c:?}").into()).into(),
                ..Default::default()
            },
            _ => IncomingLocalAddr{ 
                kind: Custom,
                ..Default::default()
             },
        }
    }
}

/// An incoming connection that has not yet begun its server-side handshake.
///
/// Consume via [`Self::accept`] / [`Self::refuse`] / [`Self::retry`] / [`Self::ignore`].
/// Each `Incoming` can only be consumed once.
#[derive_ReprC]
#[repr(opaque)]
pub struct Incoming(Mutex<Option<iroh::endpoint::Incoming>>);

impl Incoming {
    pub(crate) fn new(inner: iroh::endpoint::Incoming) -> Self {
        Self(Mutex::new(Some(inner)))
    }
}

impl Incoming {
    /// Begin the server-side handshake, producing an [`Accepting`].
    pub async fn accept(&self) -> Result<Accepting, IrohError> {
        let inner = self
            .0
            .lock()
            .await
            .take()
            .ok_or_else(|| anyhow::anyhow!("Incoming has already been consumed"))?;
        let accepting = inner.accept()?;
        Ok(Accepting::new(accepting))
    }

    /// Reject this incoming connection attempt.

    pub async fn refuse(&self) -> Result<(), IrohError> {
        let inner = self
            .0
            .lock()
            .await
            .take()
            .ok_or_else(|| anyhow::anyhow!("Incoming has already been consumed"))?;
        inner.refuse();
        Ok(())
    }

    /// Respond with a retry packet, requiring the client to retry with address
    /// validation.

    pub async fn retry(&self) -> Result<(), IrohError> {
        let inner = self
            .0
            .lock()
            .await
            .take()
            .ok_or_else(|| anyhow::anyhow!("Incoming has already been consumed"))?;
        inner
            .retry()
            .map_err(|e| anyhow::anyhow!("retry failed: {e:?}").into())
    }

    /// Drop this incoming connection without sending any reply.

    pub async fn ignore(&self) -> Result<(), IrohError> {
        let inner = self
            .0
            .lock()
            .await
            .take()
            .ok_or_else(|| anyhow::anyhow!("Incoming has already been consumed"))?;
        inner.ignore();
        Ok(())
    }

    /// The local address that received this incoming connection.

    pub async fn local_addr(&self) -> Result<IncomingLocalAddr, IrohError> {
        let guard = self.0.lock().await;
        let inner = guard
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Incoming has already been consumed"))?;
        Ok(inner.local_addr().into())
    }

    /// The remote address that originated this incoming connection.

    pub async fn remote_addr(&self) -> Result<IncomingAddr, IrohError> {
        let guard = self.0.lock().await;
        let inner = guard
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Incoming has already been consumed"))?;
        Ok(inner.remote_addr().into())
    }

    /// True if the remote address has been validated by the QUIC retry mechanism.

    pub async fn remote_addr_validated(&self) -> Result<bool, IrohError> {
        let guard = self.0.lock().await;
        let inner = guard
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Incoming has already been consumed"))?;
        Ok(inner.remote_addr_validated())
    }
}

#[ffi_export(executor=iroh_executor)]
pub async fn incoming_accept(incoming : &Incoming) -> IrohResult<repr_c::Box<Accepting>>{
    ffi_await!(async {
        IrohResult::from_result(incoming.accept().await.map(|x|
            Box::new(x).into()
        ))
    })
}

#[ffi_export(executor=iroh_executor)]
pub async fn incoming_refuse(incoming : &Incoming) -> IrohResult<()>{
    ffi_await!(async {
        IrohResult::from_result(incoming.refuse().await)
    })
}

#[ffi_export(executor=iroh_executor)]
pub async fn incoming_retry(incoming : &Incoming) -> IrohResult<()>{
    ffi_await!(async {
        IrohResult::from_result(incoming.retry().await)
    })
}

#[ffi_export(executor=iroh_executor)]
pub async fn incoming_ignore(incoming : &Incoming) -> IrohResult<()>{
    ffi_await!(async {
        IrohResult::from_result(incoming.ignore().await)
    })
}

#[ffi_export(executor=iroh_executor)]
pub async fn incoming_local_addr(incoming : &Incoming) -> IrohResult<IncomingLocalAddr>{
    ffi_await!(async {
        IrohResult::from_result(incoming.local_addr().await)
    })
}


/// A server-side handshake in progress. Await with [`Self::connect`].

#[derive_ReprC]
#[repr(opaque)]
pub struct Accepting(Mutex<Option<iroh::endpoint::Accepting>>);

impl Accepting {
    pub(crate) fn new(inner: iroh::endpoint::Accepting) -> Self {
        Self(Mutex::new(Some(inner)))
    }
}


impl Accepting {
    /// Wait for the handshake to complete, producing a [`Connection`].

    pub async fn connect(&self) -> Result<Connection, IrohError> {
        let inner = self
            .0
            .lock()
            .await
            .take()
            .ok_or_else(|| anyhow::anyhow!("Accepting has already been consumed"))?;
        let conn = inner.await.map_err(|e| anyhow::anyhow!("{e:?}"))?;
        Ok(conn.into())
    }

    /// Read the ALPN protocol from the peer's handshake data (resolves once
    /// the ClientHello has been received).

    pub async fn alpn(&self) -> Result<Vec<u8>, IrohError> {
        let mut guard = self.0.lock().await;
        let inner = guard
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("accepting has already been consumed"))?;
        Ok(inner.alpn().await?)
    }
}

#[ffi_export(executor=iroh_executor)]
pub async fn accepting_connect(accepting : &Accepting) -> IrohResult<repr_c::Box<Connection>> {
    ffi_await!( async {
        IrohResult::from_result(accepting.connect().await.map(|a| Box::new(a).into()))
    })
}

#[ffi_export(executor=iroh_executor)]
pub async fn accepting_alpn(accepting : &Accepting) -> IrohResult<repr_c::Vec<u8>> {
    ffi_await!( async {
        IrohResult::from_result(accepting.alpn().await.map(|a| a.into()))
    })
}



/// A client-side handshake in progress. Await with [`Self::connect`].
#[derive_ReprC]
#[repr(opaque)]
pub struct Connecting(Mutex<Option<iroh::endpoint::Connecting>>);

impl Connecting {
    pub(crate) fn new(inner: iroh::endpoint::Connecting) -> Self {
        Self(Mutex::new(Some(inner)))
    }
}


impl Connecting {
    /// Wait for the handshake to complete, producing a [`Connection`].

    pub async fn connect(&self) -> Result<Connection, IrohError> {
        let inner = self
            .0
            .lock()
            .await
            .take()
            .ok_or_else(|| anyhow::anyhow!("Connecting has already been consumed"))?;
        let conn = inner.await.map_err(|e| anyhow::anyhow!("{e:?}"))?;
        Ok(conn.into())
    }

    /// Read the ALPN protocol from the peer's handshake data (resolves once
    /// the server has responded with its ServerHello).

    pub async fn alpn(&self) -> Result<Vec<u8>, IrohError> {
        let mut guard = self.0.lock().await;
        let inner = guard
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Connecting has already been consumed"))?;
        Ok(inner.alpn().await?)
    }

    /// The [`EndpointId`] this connection attempt targets.

    pub async fn remote_id(&self) -> Result<EndpointId, IrohError> {
        let guard = self.0.lock().await;
        let inner = guard
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Connecting has already been consumed"))?;
        Ok(inner.remote_id().into())
    }
}

#[ffi_export(executor=iroh_executor)]
pub async fn connecting_connect(connecting : &Connecting) -> IrohResult<repr_c::Box<Connection>> {
    ffi_await!( async {
        IrohResult::from_result(connecting.connect().await.map(|a| Box::new(a).into()))
    })
}

#[ffi_export(executor=iroh_executor)]
pub async fn connecting_alpn(connecting : &Connecting) -> IrohResult<repr_c::Vec<u8>> {
    ffi_await!( async {
        IrohResult::from_result(connecting.alpn().await.map(|a| a.into()))
    })
}

#[ffi_export(executor=iroh_executor)]
pub async fn connecting_remote_id(connecting : &Connecting) -> IrohResult<EndpointId> {
    ffi_await!( async {
        IrohResult::from_result(connecting.remote_id().await)
    })
}