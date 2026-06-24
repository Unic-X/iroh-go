use iroh::RelayMode;

use iroh::{
    endpoint::{self, presets, presets::Preset as _},
    protocol::AcceptError,
};

use safer_ffi::ffi_export;
use safer_ffi::prelude::c_slice;
use safer_ffi::{derive_ReprC, prelude::repr_c};
use std::{str::FromStr, sync::{Arc, Mutex}};
use crate::watch::{self, AddrChangeCallback, HomeRelayCallback, NetworkChangeCallback, WatchHandle};
use crate::{Connecting, EndpointAddr, EndpointId, Incoming, IrohError, IrohResult, RelayConfig, SecretKey, iroh_executor};
use crate::connection::*;

#[derive_ReprC]
#[repr(opaque)]
pub struct EndpointBuilder {
    inner: Mutex<Option<iroh::endpoint::Builder>>,
}

impl EndpointBuilder {
    fn new(builder: iroh::endpoint::Builder) -> Self {
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

    fn take_inner(&self) -> Result<iroh::endpoint::Builder, IrohError> {
        self.inner
            .lock()
            .unwrap()
            .take()
            .ok_or_else(|| anyhow::anyhow!("EndpointBuilder already consumed").into())
    }
    
}

#[ffi_export]
pub fn endpoint_builder_new() -> repr_c::Box<EndpointBuilder> {
    Box::new(
        EndpointBuilder::new(iroh::endpoint::Builder::empty())
    )
    .into()
}

#[ffi_export]
pub fn endpoint_builder_free(builder: repr_c::Box<EndpointBuilder>) {
    drop(builder)
}

// Configuration Methods (Exposed to FFI)
impl EndpointBuilder {
    fn apply_n0(&self) {
        self.map(|b| presets::N0.apply(b));
    }

    fn apply_minimal(&self) {
        self.map(|b| presets::Minimal.apply(b));
    }

    fn apply_n0_disable_relay(&self) {
        self.map(|b| presets::N0DisableRelay.apply(b));
    }

    fn secret_key(&self, bytes: Vec<u8>) -> Result<(), IrohError> {

        let key: [u8; 32] = AsRef::<[u8]>::as_ref(&bytes)
            .try_into()
            .map_err(|e| anyhow::anyhow!("Invalid secret key {e}"))?;

        let key = iroh::SecretKey::from_bytes(&key);
        self.map(|b| b.secret_key(key));
        Ok(())
    }

    fn alpns(&self, alpns: Vec<Vec<u8>>) {
        self.map(|b| b.alpns(alpns));
    }

    fn relay_mode(&self, mode: &RelayMode) {
        let mode = mode.clone();
        self.map(|b| b.relay_mode(mode));
    }

    async fn bind_endpoint(&self) -> Result<Endpoint, IrohError> {
        match self.take_inner() {
            Ok(b) => {
                Ok(Endpoint::from(b.bind().await?))
            },
            Err(e) => Err(e),
        }
    }

    fn bind_addr(&self, addr: &str) -> Result<(), IrohError> {
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

/// Replay the n0 production preset (relays + discovery + crypto provider).
#[ffi_export]
pub fn apply_n0(builder: &EndpointBuilder) {
    builder.apply_n0();
}

/// Replay the minimal preset (crypto provider only, no external deps).
#[ffi_export]
pub fn apply_minimal(builder: &EndpointBuilder) {
    builder.map(|b| presets::Minimal.apply(b));
}

/// Replay the n0 preset with relays disabled.
#[ffi_export]
pub fn apply_n0_disable_relay(builder: &EndpointBuilder) {
    builder.map(|b| presets::N0DisableRelay.apply(b));
}

/// Set the advertised ALPNs.
#[ffi_export]
fn set_alpns(builder: &EndpointBuilder, alpns: repr_c::Vec<repr_c::Vec<u8>>) {
    let alpns: Vec<Vec<u8>> = alpns
    .iter().map(|v| v.as_ref().to_vec())
    .collect();
    
    builder.map(|b| b.alpns(alpns));
}

/// Set the endpoint secret key (32 bytes).
#[ffi_export]
pub fn secret_key(builder: &EndpointBuilder, key : repr_c::Vec<u8>) -> IrohResult<()> {
    let bytes = key.to_vec();
    IrohResult::from_result(builder.secret_key(bytes))
}

/// Set the relay mode.
#[ffi_export]
fn relay_mode(builder:&EndpointBuilder, mode: RelayModeFFI) {
    let mode = match mode{
        RelayModeFFI::Default => {RelayMode::Default},
        RelayModeFFI::Disabled => {RelayMode::Disabled},
        RelayModeFFI::Staging => {RelayMode::Staging},
    };

    builder.map(|b| b.relay_mode(mode));
}

/// Set the address the endpoint binds to (`host:port`).
#[ffi_export]
fn bind_addr(builder: &EndpointBuilder, addr: repr_c::String) -> IrohResult<()> {
    IrohResult::from_result(builder.bind_addr(&addr.to_string()))
}

#[ffi_export(executor=iroh_executor)]
pub async fn bind_endpoint(builder: &EndpointBuilder) -> IrohResult<repr_c::Box<Endpoint>> {
    ffi_await!( async {
        match builder.bind_endpoint().await{
            Ok(ep) => IrohResult::ok(Box::new(ep).into()),
            Err(e) => IrohResult::err(e)   
        }
    })
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

/// Configures a freshly created [`EndpointBuilder`].
///
/// This mirrors the upstream `iroh::endpoint::presets::Preset` trait and is
/// implementable from the foreign language: implement `apply` to configure the
/// builder however you like (typically calling one of the
/// [`EndpointBuilder::apply_n0`] / `apply_minimal` / `apply_n0_disable_relay`
/// baselines first, since those install the crypto provider). The built-in
/// presets are available as [`preset_n0`], [`preset_minimal`], and
/// [`preset_n0_disable_relay`].
#[ffi_export]
pub fn apply_preset(preset: &Preset, builder: &EndpointBuilder) {
    match preset {
        Preset::N0 => builder.apply_n0(),
        Preset::Minimal => builder.apply_minimal(),
        Preset::N0DisableRelay => builder.apply_n0_disable_relay(),
        _ => {}
    }
}

#[derive_ReprC]
#[repr(u8)]
#[derive(Debug,Default)]
pub enum RelayModeFFI {
    /// Disable relay servers completely.
    /// This means that neither listening nor dialing relays will be available.
    Disabled = 0,
    /// Use the default relay map, with production relay servers from n0.
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
#[derive_ReprC]
#[repr(C)]
pub struct CounterStats {
    /// The counter / gauge value.
    pub value: u32,
    /// The metric description.
    pub description: repr_c::String,
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
#[derive(Clone)]
pub struct Endpoint {
    inner: endpoint::Endpoint,
    router: Option<iroh::protocol::Router>,
}

impl Endpoint {
    pub(crate) fn raw(&self) -> &endpoint::Endpoint {
        &self.inner
    }
}

impl From<iroh::Endpoint> for Endpoint {
    fn from(value: iroh::Endpoint) -> Self {
        Self {
            inner: value,
            router: None //Not sure how to handle this 
        }
    }
}


// TODO: FFI export function for this
// /// Register a callback that fires whenever the endpoint's [`EndpointAddr`]
// /// changes (relay home rotates, IP discovered, etc.). The returned
// /// [`WatchHandle`] cancels the watcher when dropped or when its `stop()`
// /// method is called.
// pub fn watch_addr(&self, callback: Arc<dyn AddrChangeCallback>) -> Arc<WatchHandle> {
//     Arc::new(watch::spawn_watch_addr(self.inner.clone(), callback))
// }

// // /// Register a callback that fires whenever the list of relays this endpoint
// // /// is currently connected to changes.
// pub fn watch_home_relay(&self, callback: Arc<dyn HomeRelayCallback>) -> Arc<WatchHandle> {
//     Arc::new(watch::spawn_home_relay_watch(self.inner.clone(), callback))
// }

// // /// Register a callback that fires every time the underlying network stack
// // /// reports a change (interface up/down, NAT change, roaming, etc.).
// pub fn watch_network_change(
//     &self,
//     callback: Arc<dyn NetworkChangeCallback>,
// ) -> Arc<WatchHandle> {
//     Arc::new(watch::spawn_network_change_watch(
//         self.inner.clone(),
//         callback,
//     ))
// }

impl Endpoint {
    /// Bind a new endpoint with the given options.
    async fn bind(options: &EndpointOptions) -> Result<Self, IrohError> {
        let wrapper = EndpointBuilder::new(iroh::endpoint::Builder::empty());
        let preset = &options.preset;
        apply_preset(&preset,&wrapper);

        if let Some(secret_key) = &options.secret_key {
            wrapper.secret_key(secret_key.iter().as_slice().to_vec())?;
        }

        
        if let Some(alpns) = &options.alpns {
            let alpns_vec: Vec<Vec<u8>> = alpns.into_iter().map(|v| v.to_vec()).collect();
            wrapper.alpns(alpns_vec);
        }

        let relay_mode = match options.relay_mode {
            RelayModeFFI::Disabled => RelayMode::Disabled,
            RelayModeFFI::Default => RelayMode::Default,
            RelayModeFFI::Staging => RelayMode::Staging,
        };

        wrapper.relay_mode(&relay_mode);

        if let Some(addr) = &options.bind_addr {
            wrapper.bind_addr(addr)?;
        }

        let builder = wrapper.take_inner()?;
        let endpoint = builder.bind().await?;

        let router = match &options.protocols {
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
    fn id(&self) -> EndpointId {
        self.inner.id().into()
    }

    /// The [`EndpointAddr`] for this endpoint (id + currently known addresses).
    fn addr(&self) -> Box<EndpointAddr> {
        Box::new(self.inner.addr().into())
    }

    /// Connect to a remote endpoint via the given ALPN.
    async fn connect(&self, addr: &EndpointAddr, alpn: &[u8]) -> Result<Connection, IrohError> {
        let addr: iroh::EndpointAddr = addr.clone().try_into()?;
        let conn = self.inner.connect(addr, alpn).await?;
        Ok(Connection(conn))
    }

    /// Shut down the endpoint (and, if present, the protocol router).
    async fn close(&self) -> Result<(), IrohError> {
        if let Some(router) = &self.router {
            router.shutdown().await?;
        } else {
            self.inner.close().await;
        }
        Ok(())
    }

    /// Returns true if the endpoint has been closed.
    fn is_closed(&self) -> bool {
        self.inner.is_closed()
    }

    /// The [`SecretKey`] backing this endpoint's identity.
    fn secret_key(&self) -> Box<SecretKey> {
        Box::new(self.inner.secret_key().clone().into())
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

    /// Remove a relay configuration at runtime. Returns true if a relay was
    /// removed.
    pub async fn remove_relay(&self, url: String) -> Result<bool, IrohError> {
        let url = iroh::RelayUrl::from_str(&url).map_err(anyhow::Error::from)?;
        Ok(self.inner.remove_relay(&url).await.is_some())
    }

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
    pub async fn connect_pending(
        &self,
        addr: &EndpointAddr,
        alpn: &[u8],
    ) -> Result<Connecting, IrohError> {
        let addr: iroh::EndpointAddr = addr.clone().try_into()?;
        let connecting = self
            .inner
            .connect_with_opts(addr, alpn, iroh::endpoint::ConnectOptions::default())
            .await
            .map_err(|e| anyhow::anyhow!("{e:?}"))?;
        Ok(Connecting::new(connecting))
    }

    // /// Register a callback that fires whenever the endpoint's [`EndpointAddr`]
    // /// changes (relay home rotates, IP discovered, etc.). The returned
    // /// [`WatchHandle`] cancels the watcher when dropped or when its `stop()`
    // /// method is called.
    pub fn watch_addr(&self, callback: Arc<dyn AddrChangeCallback>) -> Arc<WatchHandle> {
        Arc::new(watch::spawn_watch_addr(self.inner.clone(), callback))
    }

    // /// Register a callback that fires whenever the list of relays this endpoint
    // /// is currently connected to changes.
    pub fn watch_home_relay(&self, callback: Arc<dyn HomeRelayCallback>) -> Arc<WatchHandle> {
        Arc::new(watch::spawn_home_relay_watch(self.inner.clone(), callback))
    }

    // /// Register a callback that fires every time the underlying network stack
    // /// reports a change (interface up/down, NAT change, roaming, etc.).
    pub fn watch_network_change(
        &self,
        callback: Arc<dyn NetworkChangeCallback>,
    ) -> Arc<WatchHandle> {
        Arc::new(watch::spawn_network_change_watch(
            self.inner.clone(),
            callback,
        ))
    }
}

/// Bind a new endpoint with the given options.
#[ffi_export(executor=iroh_executor)]
pub async fn endpoint_bind(options: &EndpointOptions) -> IrohResult<repr_c::Box<Endpoint>> {
    ffi_await!( async {
        Endpoint::bind(options)
        .await
        .map(|ep| IrohResult::ok(Box::new(ep).into()))
        .unwrap_or_else(|e| IrohResult::err(e))
    })
}


/// The [`EndpointId`] of this endpoint.
#[ffi_export]
pub fn endpoint_id(ep: &Endpoint) -> EndpointId {
    ep.id()
}

/// The [`EndpointAddr`] for this endpoint (id + currently known addresses).
/// MIGHT BE BUGGY?
#[ffi_export]
pub fn endpoint_addr(ep: &Endpoint) -> repr_c::Box<EndpointAddr> {
    ep.addr().into()
}

/// Connect to a remote endpoint via the given ALPN.
#[ffi_export(executor=iroh_executor)]
pub async fn endpoint_connect(ep: &Endpoint, addr: &EndpointAddr, alpn: c_slice::Ref<'_,u8>) -> IrohResult<repr_c::Box<Connection>> {
    ffi_await!( async {
        ep.connect(addr, alpn.as_slice())
        .await
        .map(|conn| IrohResult::ok(Box::new(conn).into()))
        .unwrap_or_else(|e| IrohResult::err(e))
    })
}

// Only closes doesn't free the EP 
#[ffi_export(executor=iroh_executor)]
pub async fn endpoint_close(ep: &Endpoint) -> IrohResult<()> {
   ffi_await!( async {
        IrohResult::from_result(ep.close().await)
   })
}

#[ffi_export]
pub fn endpoint_free(ep: repr_c::Box<Endpoint>) {
    drop(ep)
}

#[ffi_export]
pub fn endpoint_is_closed(ep: &Endpoint) -> bool {
    ep.is_closed()
}

/// Get The [`SecretKey`] backing this endpoint's identity.
#[ffi_export]
pub fn endpoint_secret_key(ep: &Endpoint) -> repr_c::Box<SecretKey> {
    ep.secret_key().into()
}

/// Add an external (manually-known) socket address that this endpoint is
/// reachable on. Useful when running behind a static NAT / load balancer.
#[ffi_export(executor=iroh_executor)]
pub async fn endpoint_add_external_addr(ep: &Endpoint, addr: repr_c::String) -> IrohResult<()> {
    ffi_await!( async{
        IrohResult::from_result(ep.add_external_addr(addr.into()).await)
    })
}

/// Remove a previously-added external address. Returns true if an entry was
/// removed.
#[ffi_export(executor=iroh_executor)]
pub async fn endpoint_remove_external_addr(ep: &Endpoint, addr: repr_c::String) -> IrohResult<bool> {
    ffi_await!(async{
        IrohResult::from_result(ep.remove_external_addr(addr.into()).await)
    })
}

/// The local socket addresses this endpoint is bound to.
#[ffi_export]
pub fn endpoint_bound_sockets(ep: &Endpoint) -> repr_c::Vec<repr_c::String> {
    ep.bound_sockets()
        .into_iter()
        .map(repr_c::String::from) 
        .collect::<Vec<_>>()
        .into()
}

/// Resolves once the endpoint has a usable home relay.
#[ffi_export(executor=iroh_executor)]
pub async fn endpoint_online(ep: &Endpoint) {
    ffi_await!( async {
        ep.inner.online().await;
    })
}

/// Insert (or replace) a relay configuration at runtime.
// TRY zero copy approach if any
#[ffi_export(executor=iroh_executor)]
pub async fn endpoint_insert_relay(ep: &Endpoint, config: repr_c::Box<RelayConfig>) -> IrohResult<()> {
    ffi_await!(async {
       IrohResult::from_result(ep.insert_relay((*config).clone()).await)
    })
}

/// Remove a relay configuration at runtime. Returns true if a relay was
/// removed.
#[ffi_export(executor=iroh_executor)]
pub async fn endpoint_remove_relay(ep: &Endpoint, url: repr_c::String) -> IrohResult<bool> {
    ffi_await!(async {
        IrohResult::from_result(ep.remove_relay(url.into()).await)
    })
}

/// Pull the next incoming connection attempt from the accept queue.
///
/// Returns `None` once the endpoint is closed. Use this for a custom accept
/// loop instead of (or in addition to) registering protocol handlers via
/// [`EndpointOptions::protocols`].
#[ffi_export(executor=iroh_executor)]
pub async fn endpoint_accept_next(
    ep: &Endpoint,
) -> repr_c::TaggedOption<repr_c::Box<Incoming>> {
    ffi_await!(async {
        ep.accept_next()
            .await
            .map(|incoming| Box::new(Arc::into_inner(incoming).unwrap()).into())
            .into()
    })
}

// /// Begin a connection attempt to `addr` for `alpn`, returning the
// /// in-progress [`Connecting`] state.
// ///
// /// Unlike [`Self::connect`], which awaits the handshake before returning,
// /// this exposes the pre-handshake handle so the caller can inspect ALPN or
// /// drop the attempt explicitly.
#[ffi_export(executor=iroh_executor)]
pub async fn endpoint_connect_pending(
    ep: &Endpoint,
    addr: &EndpointAddr,
    alpn: c_slice::Ref<'_,u8>,
) -> IrohResult<repr_c::Box<Connecting>> {
    ffi_await!(async{
        IrohResult::from_result(
            ep.connect_pending(addr, alpn.as_slice())
                .await
                .map(|connecting| Box::new(connecting).into())
        )
    })
}

#[cfg(test)]
mod tests {
    use safer_ffi::slice;

    use crate::{Side, runtime::GLOBAL_RUNTIME};

    use super::*;

    fn write_bytes(stream: &SendStream, bytes: &[u8]) {
        write_all(stream, slice::Ref::from(bytes)).unwrap();
    }

    const TEST_ALPN: &[u8] = b"iroh-ffi/test/0";
    const TEST_MSG: &[u8] = b"hello iroh";

    #[tokio::test]
    async fn test_bind_minimal() {
        let options = EndpointOptions {
            preset: Preset::Minimal,
            ..Default::default()
        };

        let ep = Endpoint::bind(&options).await.unwrap();

        
        assert!(!ep.raw().bound_sockets().is_empty());
        ep.raw().close();
    }

    #[tokio::test]
    async fn test_bind() {
        let options = EndpointOptions {
            preset: Preset::N0,
            ..Default::default()
        };

        let ep = Endpoint::bind(&options).await.unwrap();
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
        let ep = Endpoint::bind(&EndpointOptions {
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

    /// Full end-to-end: two endpoints, direct (no relay) connection, bi-stream
    /// echo, datagram round-trip, and connection introspection. This is the
    /// canonical connectivity test mirrored across every binding language.
    #[tokio::test]
    async fn test_connect_echo_roundtrip() {
            let server = Endpoint::bind(&EndpointOptions {
                preset: Preset::N0,
                alpns: Some(vec![TEST_ALPN.to_vec().into()].into()),
                relay_mode: RelayModeFFI::Disabled,
                ..Default::default()
            })
            .await
            .unwrap();
        
            let server_addr = server.addr();
            let server_id = server.id();

            let server_task = {
                let server = server.clone();
                tokio::spawn(async move {
                    let incoming = server.accept_next().await.expect("incoming");
                    let accepting = incoming.accept().await.unwrap();
                    let conn = accepting.connect().await.unwrap();
                    assert!(matches!(conn.side(), Side::Server));
                    assert_eq!(conn.alpn(), TEST_ALPN);

                    let bi = conn.accept_bi().await.unwrap();
                    let recv = bi.recv();
                    let send = bi.send();
                    let msg = read_to_end(&recv, 64).unwrap();
                    write_bytes(&send, &msg);
                    finish(&send).unwrap();

                    // datagram echo
                    let dg = conn.read_datagram().await.unwrap();
                    conn.send_datagram(dg).unwrap();

                    conn.closed().await;
                })
            };

            let client = Endpoint::bind(&EndpointOptions {
                preset: Preset::N0,
                relay_mode: RelayModeFFI::Disabled,
                ..Default::default()
            })
            .await
            .unwrap();

            let conn = client.connect(&server_addr, TEST_ALPN).await.unwrap();
            assert!(matches!(conn.side(), Side::Client));
            assert_eq!(conn.remote_id().to_string(), server_id.to_string());
            assert!(!conn.paths().is_empty());

            let bi = conn.open_bi().await.unwrap();
            let send = bi.send();
            let recv = bi.recv();
            write_bytes(&send, TEST_MSG);
            finish(&send).unwrap();
            let echoed = read_to_end(&recv, 64).unwrap();
            assert_eq!(&*echoed, TEST_MSG);

            conn.send_datagram(b"ping".to_vec()).unwrap();
            let pong = conn.read_datagram().await.unwrap();
            assert_eq!(pong, b"ping");

            let stats = conn.stats();
            assert!(stats.udp_tx_datagrams > 0);

            conn.close(0, b"bye").unwrap();
            server_task.await.unwrap();
            client.close().await.unwrap();
            server.close().await.unwrap();
    }

}