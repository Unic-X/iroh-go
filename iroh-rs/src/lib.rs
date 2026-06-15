use once_cell::sync::Lazy;

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::Arc;
use tokio::runtime::Runtime;

use crate::manager::IrohManager;

mod errors;
mod manager;
mod ticket;
mod endpoint;

static MANAGER: Lazy<IrohManager> = Lazy::new(|| {
    let rt = Arc::new(Runtime::new().expect("failed to create tokio runtime"));
    IrohManager::new(rt)
});

#[no_mangle]
pub extern "C" fn iroh_endpoint_new() -> i64 {
    MANAGER.create_endpoint().unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn iroh_endpoint_free(id: i64) -> bool {
    MANAGER.free_endpoint(id)
}

#[no_mangle]
pub extern "C" fn iroh_connect(endpoint: i64, endpoint_id: *const c_char) -> i64 {
    if endpoint_id.is_null() {
        return 0;
    }
    let Ok(endpoint_id) = unsafe { CStr::from_ptr(endpoint_id) }.to_str() else {
        return 0;
    };
    MANAGER.connect(endpoint, endpoint_id).unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn iroh_connection_close(conn: i64) -> bool {
    MANAGER.close_connection(conn)
}

#[no_mangle]
pub extern "C" fn iroh_endpoint_id(endpoint: i64) -> *mut c_char {
    match MANAGER.endpoint_id(endpoint) {
        Ok(node_id) => {
            let c_node_id = CString::new(node_id).unwrap();
            c_node_id.into_raw()
        }
        Err(_) => std::ptr::null_mut(),
    }
}
