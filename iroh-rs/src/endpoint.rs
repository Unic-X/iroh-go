use iroh::{RelayMode, endpoint::presets::{self, Preset}};
use std::{str::FromStr, sync::Mutex};

use crate::errors::IrohError;

pub struct EndpointBuilder {
    inner : Mutex<Option<iroh::endpoint::Builder>>
}

impl EndpointBuilder {
    pub(crate) fn new(builder: iroh::endpoint::Builder) -> Self {
        Self { inner: Mutex::new(Some(builder)) }
    }

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
}


impl EndpointBuilder {
    pub fn apply_n0(&self) {
        self.map(|b| presets::N0.apply(b));
    }

    pub fn apply_minimal(&self) {
        self.map(|b| presets::Minimal.apply(b));
    }

    /// Replay the n0 preset with relays disabled.
    pub fn apply_n0_disable_relay(&self) {
        self.map(|b| presets::N0DisableRelay.apply(b));
    }

    /// Set the endpoint secret key (32 bytes).
    pub fn secret_key(&self, bytes: Vec<u8>) -> Result<(), IrohError> {
        let key: [u8; 32] = AsRef::<[u8]>::as_ref(&bytes)
            .try_into()
            .map_err(|_| IrohError::InvalidSecretKeyLength(bytes.len()))?;
        let key = iroh::SecretKey::from_bytes(&key);
        self.map(|b| b.secret_key(key));
        Ok(())
    }

    /// Set the advertised ALPNs.
    pub fn alpns(&self, alpns: Vec<Vec<u8>>) {
        self.map(|b| b.alpns(alpns));
    }

    /// Set the relay mode.
    pub fn relay_mode(&self, mode: RelayMode) {
        self.map(|b| b.relay_mode(mode));
    }

    /// Set the address the endpoint binds to (`host:port`).
    pub fn bind_addr(&self, addr: String) -> Result<(), IrohError> {
        let socket = std::net::SocketAddr::from_str(&addr)
            .map_err(|e| IrohError::InvalidSocketAddr(e.to_string()))?;

        let builder: iroh::endpoint::Builder = self.take_inner()?;

        let builder = builder
            .bind_addr(socket)
            .map_err(|e| IrohError::BindAddrError(e.to_string()))?;
        *self.inner.lock().unwrap() = Some(builder);
        Ok(())
    }
}