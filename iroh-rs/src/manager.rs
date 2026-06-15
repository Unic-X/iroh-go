use crate::errors::IrohError;
use iroh::{endpoint::Connection, Endpoint, EndpointId};
use std::sync::Arc;
use tokio::runtime::Runtime;

const ALPN: &[u8] = b"go-iroh/0";

/// Internal wrapper for an Endpoint.
/// This struct is allocated on the heap and passed to Go as an opaque pointer.
pub struct EndpointHandle {
    pub runtime: Arc<Runtime>,
    pub endpoint: Endpoint,
}

pub struct ConnectionHandle {
    pub connection: Connection,
}


impl EndpointHandle {
    pub fn new(runtime: Arc<Runtime>, endpoint: Endpoint) -> Self {
        Self { runtime, endpoint }
    }

    pub fn create_connection(&self, endpoint_id: EndpointId) -> Result<ConnectionHandle, IrohError> {
        let conn = self.runtime.block_on(async {
            self.endpoint.connect(endpoint_id, ALPN).await
        })?;
        Ok(ConnectionHandle { connection: conn })
    }

    pub fn id(&self) -> String {
        self.endpoint.id().to_string()
    }
}