use iroh::endpoint::{presets, Connection};
use iroh::{Endpoint, EndpointId};
use once_cell::sync::Lazy;
use std::collections::HashMap;

use std::ffi::CStr;
use std::os::raw::c_char;

use std::str::FromStr;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Mutex;
use tokio::runtime::Runtime;

mod connect;

static NEXT_ID: AtomicI64 = AtomicI64::new(1);

static RUNTIME: Lazy<Runtime> =
    Lazy::new(|| Runtime::new().expect("failed to create tokio runtime"));

static ENDPOINTS: Lazy<Mutex<HashMap<i64, Endpoint>>> = Lazy::new(|| Mutex::new(HashMap::new()));

static CONNECTIONS: Lazy<Mutex<HashMap<i64, Connection>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

const ALPN: &[u8] = b"go-iroh/0";

#[no_mangle]
pub extern "C" fn iroh_endpoint_new() -> i64 {
    let Ok(endpoint) = RUNTIME.block_on(async {
        Endpoint::builder(presets::N0)
            .alpns(vec![ALPN.to_vec()])
            .bind()
            .await
    }) else {
        return 0;
    };

    let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
    ENDPOINTS.lock().unwrap().insert(id, endpoint);

    id
}

#[no_mangle]
pub extern "C" fn iroh_endpoint_free(id: i64) -> bool {
    match ENDPOINTS.lock().unwrap().remove(&id) {
        Some(_) => true,
        None => false,
    }
}

#[no_mangle]
pub extern "C" fn iroh_connect(endpoint: i64, node_id: *const c_char) -> i64 {
    if node_id.is_null() {
        return 0;
    }

    let Ok(node_id) = unsafe { CStr::from_ptr(node_id) }.to_str() else {
        return 0;
    };

    let Ok(node_id) = EndpointId::from_str(node_id) else {
        return 0;
    };

    let Some(endpoint) = ENDPOINTS.lock().unwrap().get(&endpoint).cloned() else {
        return 0;
    };

    let Ok(conn) = RUNTIME.block_on(async { endpoint.connect(node_id, ALPN).await }) else {
        return 0;
    };

    let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
    CONNECTIONS.lock().unwrap().insert(id, conn);

    id
}

#[no_mangle]
pub extern "C" fn iroh_connection_close(conn: i64) -> bool {
    match CONNECTIONS.lock().unwrap().remove(&conn) {
        Some(_) => true,
        None => false,
    }
}
