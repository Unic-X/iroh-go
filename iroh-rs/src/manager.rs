use crate::errors::{self, IrohError};
use iroh::endpoint::presets;
use iroh::{endpoint::Connection, Endpoint, EndpointId};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;

const ALPN: &[u8] = b"go-iroh/0";

pub struct IrohManager {
    runtime: Arc<Runtime>,
    endpoints: Mutex<HashMap<i64, Endpoint>>,
    connections: Mutex<HashMap<i64, Connection>>,
    next_id: AtomicI64,
}

impl IrohManager {
    pub fn new(runtime: Arc<Runtime>) -> Self {
        Self {
            runtime,
            endpoints: Mutex::new(HashMap::new()),
            connections: Mutex::new(HashMap::new()),
            next_id: AtomicI64::new(1),
        }
    }

    fn alloc_id(&self) -> i64 {
        self.next_id.fetch_add(1, Ordering::SeqCst)
    }

    pub fn create_endpoint(&self) -> Result<i64, errors::IrohError> {
        let endpoint = self.runtime.block_on(async {
            Endpoint::builder(presets::N0)
                .alpns(vec![ALPN.to_vec()])
                .bind()
                .await
        })?;

        let id = self.alloc_id();
        self.endpoints.lock().unwrap().insert(id, endpoint);
        Ok(id)
    }

    pub fn free_endpoint(&self, id: i64) -> bool {
        self.endpoints.lock().unwrap().remove(&id).is_some()
    }

    pub fn connect(&self, endpoint_id: i64, node_id_str: &str) -> Result<i64, errors::IrohError> {
        let node_id = EndpointId::from_str(node_id_str)
            .map_err(|e| errors::IrohError::InvalidNodeId(e.to_string()))?;

        let endpoint = self
            .endpoints
            .lock()
            .unwrap()
            .get(&endpoint_id)
            .cloned()
            .ok_or(errors::IrohError::EndpointNotFound)?;

        let conn = self
            .runtime
            .block_on(async { endpoint.connect(node_id, ALPN).await })?;

        let id = self.alloc_id();
        self.connections.lock().unwrap().insert(id, conn);
        Ok(id)
    }

    pub fn close_connection(&self, conn_id: i64) -> bool {
        self.connections.lock().unwrap().remove(&conn_id).is_some()
    }

    pub fn endpoint_node_id(&self, endpoint_id: i64) -> Result<String, IrohError> {
        let endpoint = self
            .endpoints
            .lock()
            .unwrap()
            .get(&endpoint_id)
            .cloned()
            .ok_or(IrohError::EndpointNotFound)?;
        Ok(endpoint.id().to_string())
    }

}
