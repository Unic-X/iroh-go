use crate::errors::{self, IrohError};
use iroh::endpoint::presets;
use iroh::{endpoint::Connection, Endpoint, EndpointId};
use slab::Slab;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;

const ALPN: &[u8] = b"go-iroh/0";

pub struct IrohManager {
    runtime: Arc<Runtime>,
    endpoints: Mutex<Slab<Endpoint>>,
    connections: Mutex<Slab<Connection>>,
}

impl IrohManager {
    pub fn new(runtime: Arc<Runtime>) -> Self {
        Self {
            runtime,
            endpoints: Mutex::new(Slab::new()),
            connections: Mutex::new(Slab::new()),
        }
    }

    pub fn create_endpoint(&self) -> Result<i64, errors::IrohError> {
        let endpoint = self.runtime.block_on(async {
            Endpoint::builder(presets::N0)
                .alpns(vec![ALPN.to_vec()])
                .bind()
                .await
        })?;

        
        let id = self.endpoints.lock().unwrap().insert(endpoint);
        Ok(id as i64)
    }

    pub fn free_endpoint(&self, id: i64) -> bool {
        let mut endpoints = self.endpoints.lock().unwrap();

        if endpoints.contains(id as usize) {
            endpoints.remove(id as usize);
            true
        } else {
            false
        }
    }


    // Connect to Endpoint using EndpointId key from its z-base-32 encoding.
    //
    // Returns the connection ID.
    pub fn connect(&self, endpoint_id: i64, endpoint_id_str: &str) -> Result<i64, errors::IrohError> {
        let node_id = EndpointId::from_str(endpoint_id_str)
            .map_err(|e| errors::IrohError::InvalidNodeId(e.to_string()))?;

        let endpoint = self
            .endpoints
            .lock()
            .unwrap()
            .get(endpoint_id as usize)
            .cloned()
            .ok_or(errors::IrohError::EndpointNotFound)?;

        let conn = self
            .runtime
            .block_on(async { endpoint.connect(node_id, ALPN).await })?;

        let id = self.connections.lock().unwrap().insert(conn);
        Ok(id as i64)
    }

    pub fn close_connection(&self, conn_id: i64) -> bool {
        let mut connections = self.connections.lock().unwrap();

        if connections.contains(conn_id as usize) {
            connections.remove(conn_id as usize);
            true
        } else {
            false
        }
    }

    pub fn endpoint_id(&self, endpoint_id: i64) -> Result<String, IrohError> {
        let endpoint = self
        .endpoints
        .lock()
        .unwrap()
        .get(endpoint_id as usize)
        .cloned()
        .ok_or(errors::IrohError::EndpointNotFound)?;
        Ok(endpoint.id().to_string())
    }

}
