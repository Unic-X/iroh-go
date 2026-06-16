use iroh::{RelayMode, endpoint::{self, presets::{self, Preset as _}}, protocol::AcceptError};
use safer_ffi::{derive_ReprC, prelude::repr_c};
use std::{str::FromStr, sync::{Arc, Mutex}};
use crate::errors::IrohError;

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
            .ok_or_else(|| anyhow::anyhow!("endpoint builder already consumed").into())
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

#[derive_ReprC]
#[repr(u8)]
pub enum Preset{
    None = 0,
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

#[derive_ReprC]
#[repr(u8)]
pub enum RelayModeFFI {
    /// Disable relay servers completely.
    /// This means that neither listening nor dialing relays will be available.
    Disabled = 0,
    /// Use the default relay map, with production relay servers from n0.
    ///
    /// See [`crate::defaults::prod`] for the severs used.
    Default = 1,
    /// Use the staging relay servers from n0.
    Staging = 2,
    // TODO allow Use of custom relay map.
    // Custom(RelayMap),
}

#[derive_ReprC]
#[repr(C)]
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

#[derive_ReprC]
#[repr(opaque)]
pub struct Connection(endpoint::Connection);


pub type AcceptFn = extern "C" fn(conn: repr_c::Box<Connection>) -> bool;
pub type ShutdownFn = extern "C" fn();

#[derive(Debug)]
#[derive_ReprC]
#[repr(C)]
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
}
