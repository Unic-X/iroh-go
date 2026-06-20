use iroh::RelayMode;

use iroh::{
    endpoint::{self, presets, presets::Preset as _},
    protocol::AcceptError,
};

use safer_ffi::{derive_ReprC, prelude::repr_c};
use std::{str::FromStr, sync::{Arc, Mutex}};
use crate::{EndpointAddr, EndpointId, Incoming, IrohError, SecretKey, Connection, RelayConfig};

#[derive_ReprC]
#[repr(opaque)]
pub struct EndpointBuilder {
    inner: Mutex<Option<iroh::endpoint::Builder>>,
}

impl EndpointBuilder {
    pub(crate) fn new(builder: iroh::endpoint::Builder) -> Self {
        Self { inner: Mutex::new(Some(builder)) }
    }

    // Internal helper remains private
    fn map<F>(&self, f: F)
    where
        F: FnOnce(iroh::endpoint::Builder) -> iroh::endpoint::Builder,
    {
        let mut guard = self.inner.lock().unwrap();
        let builder = guard.take().expect("EndpointBuilder consumed");
        *guard = Some(f(builder));
    }

    pub(crate) fn take_inner(&self) -> Result<iroh::endpoint::Builder, IrohError> {
        self.inner
            .lock()
            .unwrap()
            .take()
            .ok_or_else(|| anyhow::anyhow!("EndpointBuilder already consumed").into())
    }
    
    // Helper to extract the builder for lib.rs to bind asynchronously

}

// Configuration Methods (Exposed to FFI)
impl EndpointBuilder {
    pub fn apply_n0(&self) {
        self.map(|b| presets::N0.apply(b));
    }

    pub fn apply_minimal(&self) {
        self.map(|b| presets::Minimal.apply(b));
    }

    pub fn apply_n0_disable_relay(&self) {
        self.map(|b| presets::N0DisableRelay.apply(b));
    }

    pub fn secret_key(&self, bytes: Vec<u8>) -> Result<(), IrohError> {

        let key: [u8; 32] = AsRef::<[u8]>::as_ref(&bytes)
            .try_into()
            .map_err(|e| anyhow::anyhow!("Invalid secret key {e}"))?;

        let key = iroh::SecretKey::from_bytes(&key);
        self.map(|b| b.secret_key(key));
        Ok(())
    }

    pub fn alpns(&self, alpns: Vec<Vec<u8>>) {
        self.map(|b| b.alpns(alpns));
    }

    pub fn relay_mode(&self, mode: &RelayMode) {
        let mode = mode.clone();
        self.map(|b| b.relay_mode(mode));
    }

    pub fn bind_addr(&self, addr: String) -> Result<(), IrohError> {
        let socket = std::net::SocketAddr::from_str(&addr)
            .map_err(|e| anyhow::anyhow!("invalid binding address {e}"))?;
        
        // Special handling for bind_addr since it consumes the builder in your original code?
        // Your original code did: let builder = self.take_inner()?; ... *guard = Some(builder);
        // This is safe if we use the map pattern, but bind_addr in iroh might consume self.
        // If iroh::Builder::bind_addr takes self, we must use take_inner/replace pattern.
        
        let mut guard = self.inner.lock().unwrap();
        let mut builder = guard.take().expect("EndpointBuilder consumed");
        builder = builder.bind_addr(socket)
            .map_err(|e| anyhow::anyhow!("invalid binding address {e}"))?;
        *guard = Some(builder);
        Ok(())
    }
}

#[derive(Debug, Default, Clone)]
#[derive_ReprC]
#[repr(u8)]
pub enum Preset{
    None = 0, 
    
    #[default]
    N0 = 1,
    Minimal = 2,
    N0DisableRelay = 3,
}

impl Preset {
    pub fn apply(&self, builder: Arc<EndpointBuilder>) {
        match self {
            Preset::N0 => builder.apply_n0(),
            Preset::Minimal => builder.apply_minimal(),
            Preset::N0DisableRelay => builder.apply_n0_disable_relay(),
            _ => {}
        }
    }
}

#[derive(Debug,Default)]
#[derive_ReprC]
#[repr(u8)]

pub enum RelayModeFFI {
    /// Disable relay servers completely.
    /// This means that neither listening nor dialing relays will be available.
    Disabled = 0,
    /// Use the default relay map, with production relay servers from n0.
    ///
    /// See [`crate::defaults::prod`] for the severs used.
    #[default]
    Default = 1,
    /// Use the staging relay servers from n0.
    Staging = 2,
    // TODO allow Use of custom relay map.
    // Custom(RelayMap),
}
#[derive_ReprC]
#[repr(C)]
#[derive(Debug, Default)]
pub struct EndpointOptions {
    /// Preset that configures the endpoint builder. Defaults to [`Preset::N0`].
    
    pub preset: Preset,
    /// Override the address the endpoint binds to. Accepts any standard
    /// `host:port` form (IPv4 or IPv6).
    pub bind_addr: Option<repr_c::String>,
    /// Provide a specific secret key, identifying this endpoint. Must be 32 bytes long.
    pub secret_key: Option<repr_c::Vec<u8>>,
    /// ALPN protocols advertised on the underlying TLS handshake. Independent of
    /// the per-protocol handlers in `protocols`; useful for client-only setups
    /// or for declaring extra ALPNs.
    pub alpns: Option<repr_c::Vec<repr_c::Vec<u8>>>,
    /// Override which relays the endpoint uses. Defaults to whatever the
    /// chosen [`Preset`] configures.
    pub relay_mode: RelayModeFFI,
    // Custom protocols to accept on this endpoint, keyed by ALPN. If provided,
    // an internal router is spawned to dispatch incoming connections to the
    // supplied handlers.
     
    //TODO:CustomProtocol Creator 
    pub protocols: Option<repr_c::Vec<ProtocolHandler>>,
}

pub type AcceptFn = extern "C" fn(conn: repr_c::Box<Connection>) -> bool;
pub type ShutdownFn = extern "C" fn();

#[derive_ReprC]
#[repr(C)]
#[derive(Debug)]
pub struct ProtocolHandler {
    pub alpn: repr_c::Vec<u8>,
    pub on_accept: AcceptFn,
    pub on_shutdown: ShutdownFn,
}

#[derive(Debug, Clone)]
struct FfiProtocolWrapper {
    on_accept: AcceptFn,
    on_shutdown: ShutdownFn,
}

impl iroh::protocol::ProtocolHandler for FfiProtocolWrapper {
    async fn accept(
        &self,
        conn: iroh::endpoint::Connection,
    ) -> Result<(), AcceptError> {
        let conn = Box::new(Connection(conn)).into();

        if (self.on_accept)(conn) {
            Ok(())
        } else {
            Err(AcceptError::from_err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "connection rejected by FFI protocol handler",
            )))
        }
    }

    async fn shutdown(&self) {
        (self.on_shutdown)();
    }
}

/// A snapshot value for a single endpoint metric.
pub struct CounterStats {
    /// The counter / gauge value.
    pub value: u32,
    /// The metric description.
    pub description: String,
}

/// Flat snapshot of the headline numbers from `noq::ConnectionStats`.
///
/// Counters are `i64` (not `u64`) so Kotlin sees `Long`, not `ULong`.
#[derive_ReprC]
#[repr(C)]
pub struct ConnectionStats {
    /// Total UDP datagrams transmitted.
    pub udp_tx_datagrams: i64,
    /// Total UDP bytes transmitted.
    pub udp_tx_bytes: i64,
    /// Total UDP datagrams received.
    pub udp_rx_datagrams: i64,
    /// Total UDP bytes received.
    pub udp_rx_bytes: i64,
    /// Total packets considered lost.
    pub lost_packets: i64,
    /// Total bytes considered lost.
    pub lost_bytes: i64,
}

#[derive_ReprC]
#[repr(opaque)]
pub struct Endpoint {
    inner: endpoint::Endpoint,
    router: Option<iroh::protocol::Router>,
}

impl Endpoint {
    pub fn new(ep: endpoint::Endpoint) -> Self {
        Endpoint {
            inner: ep,
            router: None,
        }
    }

    pub(crate) fn raw(&self) -> &endpoint::Endpoint {
        &self.inner
    }
}

impl Endpoint {
    pub async fn bind(options: EndpointOptions) -> Result<Self, IrohError> {
        let wrapper = Arc::new(EndpointBuilder::new(iroh::endpoint::Builder::empty()));
        let preset = options.preset;
        preset.apply(wrapper.clone());

        if let Some(secret_key) = options.secret_key {
            wrapper.secret_key(secret_key.into())?;
        }

        
        if let Some(alpns) = options.alpns {
            let alpns_vec: Vec<Vec<u8>> = alpns.into_iter().map(|v| v.to_vec()).collect();
            wrapper.alpns(alpns_vec);
        }

        let relay_mode = match options.relay_mode {
            RelayModeFFI::Disabled => RelayMode::Disabled,
            RelayModeFFI::Default => RelayMode::Default,
            RelayModeFFI::Staging => RelayMode::Staging,
        };

        wrapper.relay_mode(&relay_mode);

        if let Some(addr) = options.bind_addr {
            wrapper.bind_addr(addr.into())?;
        }

        let builder = wrapper.take_inner()?;
        let endpoint = builder.bind().await?;

        let router = match options.protocols {
            Some(protocols) if !protocols.is_empty() => {
                let mut router_builder =
                    iroh::protocol::Router::builder(endpoint.clone());

                for protocol in protocols.into_iter() {
                    let wrapper = FfiProtocolWrapper {
                        on_accept: protocol.on_accept,
                        on_shutdown: protocol.on_shutdown,
                    };

                    router_builder =
                        router_builder.accept(protocol.alpn.to_vec(), wrapper);
                }

                Some(router_builder.spawn())
            }
            _ => None,
        };

        Ok(Endpoint {
            inner: endpoint,
            router,
        })
    }

    /// The [`EndpointId`] of this endpoint.
    pub fn id(&self) -> Arc<EndpointId> {
        Arc::new(self.inner.id().into())
    }

    /// The [`EndpointAddr`] for this endpoint (id + currently known addresses).
    pub fn addr(&self) -> Arc<EndpointAddr> {
        Arc::new(self.inner.addr().into())
    }

    /// Connect to a remote endpoint via the given ALPN.
    pub async fn connect(&self, addr: &EndpointAddr, alpn: &[u8]) -> Result<Connection, IrohError> {
        let addr: iroh::EndpointAddr = addr.clone().try_into()?;
        let conn = self.inner.connect(addr, alpn).await?;
        Ok(Connection(conn))
    }


    /// Shut down the endpoint (and, if present, the protocol router).
    pub async fn close(&self) -> Result<(), IrohError> {
        if let Some(router) = &self.router {
            router.shutdown().await?;
        } else {
            self.inner.close().await;
        }
        Ok(())
    }

    /// Returns true if the endpoint has been closed.
    pub fn is_closed(&self) -> bool {
        self.inner.is_closed()
    }


    /// The [`SecretKey`] backing this endpoint's identity.
    pub fn secret_key(&self) -> Arc<SecretKey> {
        Arc::new(self.inner.secret_key().clone().into())
    }


    /// Add an external (manually-known) socket address that this endpoint is
    /// reachable on. Useful when running behind a static NAT / load balancer.
    pub async fn add_external_addr(&self, addr: String) -> Result<(), IrohError> {
        let socket = std::net::SocketAddr::from_str(&addr).map_err(anyhow::Error::from)?;
        self.inner.add_external_addr(socket).await;
        Ok(())
    }


    /// Remove a previously-added external address. Returns true if an entry was
    /// removed.
    pub async fn remove_external_addr(&self, addr: String) -> Result<bool, IrohError> {
        let socket = std::net::SocketAddr::from_str(&addr).map_err(anyhow::Error::from)?;
        Ok(self.inner.remove_external_addr(&socket).await)
    }

    /// The local socket addresses this endpoint is bound to.
    pub fn bound_sockets(&self) -> Vec<String> {
        self.inner
            .bound_sockets()
            .into_iter()
            .map(|a| a.to_string())
            .collect()
    }

    /// Resolves once the endpoint has a usable home relay.
    pub async fn online(&self) {
        self.inner.online().await;
    }

    /// Insert (or replace) a relay configuration at runtime.
    pub async fn insert_relay(&self, config: RelayConfig) -> Result<(), IrohError> {
        let config: iroh::RelayConfig = config.try_into()?;
        let url = config.url.clone();
        self.inner.insert_relay(url, Arc::new(config)).await;
        Ok(())
    }

    // /// Remove a relay configuration at runtime. Returns true if a relay was
    // /// removed.
    // pub async fn remove_relay(&self, url: String) -> Result<bool, IrohError> {
    //     let url = iroh::RelayUrl::from_str(&url).map_err(anyhow::Error::from)?;
    //     Ok(self.inner.remove_relay(&url).await.is_some())
    // }

    /// Pull the next incoming connection attempt from the accept queue.
    ///
    /// Returns `None` once the endpoint is closed. Use this for a custom accept
    /// loop instead of (or in addition to) registering protocol handlers via
    /// [`EndpointOptions::protocols`].
    pub async fn accept_next(&self) -> Option<Arc<Incoming>> {
        let incoming = self.inner.accept().await?;
        Some(Arc::new(Incoming::new(incoming)))
    }

    // /// Begin a connection attempt to `addr` for `alpn`, returning the
    // /// in-progress [`Connecting`] state.
    // ///
    // /// Unlike [`Self::connect`], which awaits the handshake before returning,
    // /// this exposes the pre-handshake handle so the caller can inspect ALPN or
    // /// drop the attempt explicitly.
    // pub async fn connect_pending(
    //     &self,
    //     addr: &EndpointAddr,
    //     alpn: &[u8],
    // ) -> Result<Connecting, IrohError> {
    //     let addr: iroh::EndpointAddr = addr.clone().try_into()?;
    //     let connecting = self
    //         .inner
    //         .connect_with_opts(addr, alpn, iroh::endpoint::ConnectOptions::default())
    //         .await
    //         .map_err(|e| anyhow::anyhow!("{e:?}"))?;
    //     Ok(Connecting::new(connecting))
    // }

    // /// Register a callback that fires whenever the endpoint's [`EndpointAddr`]
    // /// changes (relay home rotates, IP discovered, etc.). The returned
    // /// [`WatchHandle`] cancels the watcher when dropped or when its `stop()`
    // /// method is called.
    // pub fn watch_addr(&self, callback: Arc<dyn AddrChangeCallback>) -> Arc<WatchHandle> {
    //     Arc::new(watch::spawn_watch_addr(self.inner.clone(), callback))
    // }

    // /// Register a callback that fires whenever the list of relays this endpoint
    // /// is currently connected to changes.
    // pub fn watch_home_relay(&self, callback: Arc<dyn HomeRelayCallback>) -> Arc<WatchHandle> {
    //     Arc::new(watch::spawn_home_relay_watch(self.inner.clone(), callback))
    // }

    // /// Register a callback that fires every time the underlying network stack
    // /// reports a change (interface up/down, NAT change, roaming, etc.).
    // pub fn watch_network_change(
    //     &self,
    //     callback: Arc<dyn NetworkChangeCallback>,
    // ) -> Arc<WatchHandle> {
    //     Arc::new(watch::spawn_network_change_watch(
    //         self.inner.clone(),
    //         callback,
    //     ))
    // }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_bind_minimal() {
        let options = EndpointOptions {
            preset: Preset::Minimal,
            ..Default::default()
        };

        let ep = Endpoint::bind(options).await.unwrap();

        
        assert!(!ep.raw().bound_sockets().is_empty());
        ep.raw().close();
    }

    #[tokio::test]
    async fn test_bind() {
        let options = EndpointOptions {
            preset: Preset::N0,
            ..Default::default()
        };

        let ep = Endpoint::bind(options).await.unwrap();
        let id = ep.raw().id();
        println!("{id}");
        
        assert!(!ep.raw().bound_sockets().is_empty(), "should have bound sockets");
        let secret = ep.raw().secret_key();
        assert_eq!(secret.public().as_bytes(), id.as_bytes());
        ep.raw().close().await;
        assert!(ep.raw().is_closed());
    }

    #[tokio::test]
    async fn test_side_paths_compile() {
        // Surface-level smoke test: the new accept/path types must compile and
        // be callable. End-to-end connection establishment lives in higher-level
        // language-binding tests.
        let ep = Endpoint::bind(EndpointOptions {
            preset: Preset::N0,
            ..Default::default()
        })
        .await
        .unwrap();
        // accept_next polled with timeout: just confirm it returns a future of
        // Option<Arc<Incoming>>.
        let timeout = tokio::time::sleep(std::time::Duration::from_millis(10));
        tokio::pin!(timeout);
        tokio::select! {
            _ = &mut timeout => {}
            _next = ep.accept_next() => {}
        }
        ep.raw().close().await;
    }

}