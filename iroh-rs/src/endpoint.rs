use iroh::{RelayMode, endpoint::{self, presets::{self, Preset}}};
use std::{collections::HashMap, str::FromStr, sync::{Arc, Mutex}};
use crate::errors::IrohError;

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
            .ok_or_else(|| IrohError::EndpointBuilderConsumed)
    }
    
    // Helper to extract the builder for lib.rs to bind asynchronously
    pub fn into_builder(self) -> Result<iroh::endpoint::Builder, IrohError> {
        self.take_inner()
    }
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

    pub fn secret_key(&self, bytes: &[u8; 32]) -> Result<(), IrohError> {
        let key = iroh::SecretKey::from_bytes(bytes);
        self.map(|b| b.secret_key(key));
        Ok(())
    }

    pub fn alpns(&self, alpns: &[Vec<u8>]) {
        // Clone the vecs for the closure
        let alpns_clone = alpns.to_vec();
        self.map(|b| b.alpns(alpns_clone));
    }

    pub fn relay_mode(&self, mode: RelayMode) {
        self.map(|b| b.relay_mode(mode));
    }

    pub fn bind_addr(&self, addr: &str) -> Result<(), IrohError> {
        let socket = std::net::SocketAddr::from_str(addr)
            .map_err(|e| IrohError::InvalidSocketAddr(e.to_string()))?;
        
        // Special handling for bind_addr since it consumes the builder in your original code?
        // Your original code did: let builder = self.take_inner()?; ... *guard = Some(builder);
        // This is safe if we use the map pattern, but bind_addr in iroh might consume self.
        // If iroh::Builder::bind_addr takes self, we must use take_inner/replace pattern.
        
        let mut guard = self.inner.lock().unwrap();
        let mut builder = guard.take().expect("EndpointBuilder consumed");
        builder = builder.bind_addr(socket)
            .map_err(|e| IrohError::BindAddrError(e.to_string()))?;
        *guard = Some(builder);
        Ok(())
    }
}


pub struct EndpointOptions {
    /// Preset that configures the endpoint builder. Defaults to [`preset_n0`].
    /// Implement the [`Preset`] trait in your language for full control.
    pub preset: Option<Arc<dyn Preset>>,
    /// Override the address the endpoint binds to. Accepts any standard
    /// `host:port` form (IPv4 or IPv6).
    pub bind_addr: Option<String>,
    /// Provide a specific secret key, identifying this endpoint. Must be 32 bytes long.
    pub secret_key: Option<Vec<u8>>,
    /// ALPN protocols advertised on the underlying TLS handshake. Independent of
    /// the per-protocol handlers in `protocols`; useful for client-only setups
    /// or for declaring extra ALPNs.
    pub alpns: Option<Vec<Vec<u8>>>,
    /// Override which relays the endpoint uses. Defaults to whatever the
    /// chosen [`Preset`] configures.
    pub relay_mode: Option<Arc<RelayMode>>,
    // Custom protocols to accept on this endpoint, keyed by ALPN. If provided,
    // an internal router is spawned to dispatch incoming connections to the
    // supplied handlers.
     
    //TODO:CustomProtocol Creator 
    // pub protocols: Option<HashMap<Vec<u8>, Arc<dyn ProtocolCreator>>>,
}

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
        todo!()
    }
}