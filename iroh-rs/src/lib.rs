use std::ffi::CStr;
use std::os::raw::{c_char, c_void, c_uchar};
use std::sync::Arc;
use once_cell::sync::Lazy;
use tokio::runtime::Runtime;

use crate::manager::{EndpointHandle, ConnectionHandle};
use crate::endpoint::EndpointBuilder;
use iroh::RelayMode;

mod errors;
mod manager;
mod endpoint;
mod ticket;


static GLOBAL_RUNTIME: Lazy<Arc<Runtime>> = Lazy::new(|| {
    Arc::new(Runtime::new().expect("failed to create runtime"))
});

#[no_mangle]
pub unsafe extern "C" fn iroh_builder_new() -> *mut c_void {
    let builder = EndpointBuilder::new(iroh::endpoint::Builder::empty());
    Box::into_raw(Box::new(builder)) as *mut c_void
}

/// Applies the N0 preset to the builder.
#[no_mangle]
pub unsafe extern "C" fn iroh_builder_apply_n0(ptr: *mut c_void){
    if ptr.is_null() { return; }
    let handle = &*(ptr as *const EndpointBuilder);
    handle.apply_n0();
}

/// Applies the Minimal preset.
#[no_mangle]
pub unsafe extern "C" fn iroh_builder_apply_minimal(ptr: *mut c_void) {
    if ptr.is_null() { return; }
    let handle = &*(ptr as *const EndpointBuilder);
    handle.apply_minimal();
}

/// Sets the secret key. Expects a byte array and length.
#[no_mangle]
pub unsafe extern "C" fn iroh_builder_secret_key(
    ptr: *mut c_void,
    key_bytes: *const c_uchar,
)-> bool {
    if ptr.is_null() || key_bytes.is_null() {
        return false;
    }
    let handle = &*(ptr as *const EndpointBuilder);
    let key = &*(key_bytes as *const [u8; 32]);
    handle.secret_key(key).is_ok()
}

#[no_mangle]
pub unsafe extern "C" fn iroh_builder_add_alpn(ptr: *mut c_void, alpn: *const c_char) -> bool {
    if ptr.is_null() || alpn.is_null() {
        return false;
    }
    let handle = &*(ptr as *const EndpointBuilder);
    let s = match CStr::from_ptr(alpn).to_str() {
        Ok(s) => s,
        Err(_) => return false,
    };
    
    handle.alpns(&[s.as_bytes().to_vec()]);
    true
}

/// Sets the relay mode (0=Default, 1=Disabled, 2=Custom - requires extra logic).
#[no_mangle]
pub unsafe extern "C" fn iroh_builder_relay_mode(ptr: *mut c_void, mode: u8) {
    if ptr.is_null() { return; }
    let handle = &*(ptr as *const EndpointBuilder);

    // For now, we only support Default mode. Custom mode would require more complex configuration.
    _ = mode;
    handle.relay_mode(RelayMode::Default);
}

/// Sets the bind address (e.g., "0.0.0.0:0").
#[no_mangle]
pub unsafe extern "C" fn iroh_builder_bind_addr(ptr: *mut c_void, addr: *const c_char) -> bool {
    if ptr.is_null() || addr.is_null() { return false; }
    let handle = &*(ptr.cast::<EndpointBuilder>());
    let s = match CStr::from_ptr(addr).to_str() {
        Ok(s) => s,
        Err(_) => return false,
    };
    handle.bind_addr(s).is_ok()
}

#[no_mangle]
pub unsafe extern "C" fn iroh_builder_free(ptr: *mut c_void) {
    if ptr.is_null() { return; }
    let _ = Box::from_raw(ptr as *mut EndpointBuilder);
}

//TODO: Implement endpoint creation 
// TODO: Implement Endpoint 

#[no_mangle]
pub unsafe extern "C" fn iroh_endpoint_free(ptr: *mut c_void) {
    if ptr.is_null() { return; }
    let _ = Box::from_raw(ptr as *mut EndpointHandle);
}


#[no_mangle]
pub unsafe extern "C" fn iroh_connection_free(ptr: *mut c_void) {
    if ptr.is_null() { return; }
    let _ = Box::from_raw(ptr as *mut ConnectionHandle);
}
