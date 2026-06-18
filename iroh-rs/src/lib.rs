use safer_ffi::prelude::*;
use safer_ffi::{derive_ReprC, prelude::repr_c};
use crate::{endpoint::{Endpoint, EndpointBuilder, EndpointOptions}, runtime::GLOBAL_RUNTIME};
use iroh::RelayMode;

mod errors;
mod endpoint;
mod runtime;
mod key;
mod net;
mod ticket;
mod accept;
mod connection;
mod relay;


pub use self::{
    accept::*, endpoint::*, errors::*, key::*, net::*, ticket::*, connection::*, relay::*,
};

#[ffi_export]
fn iroh_builder_new() -> repr_c::Box<EndpointBuilder> {
    Box::new(
        EndpointBuilder::new(iroh::endpoint::Builder::empty())
    )
    .into()
}

#[ffi_export]
fn iroh_endpoint_bind(options: repr_c::Box<EndpointOptions>) -> Option<repr_c::Box<Endpoint>> {
    let options: Box<EndpointOptions> = options.into();

    match GLOBAL_RUNTIME.block_on(Endpoint::bind(*options)) {
        Ok(ep) => Some(Box::new(ep).into()),
        Err(e) => {
            // TODO: Set error on out_err
            None
        }
    }
}


#[ffi_export]
fn iroh_builder_apply_n0(
    builder: &EndpointBuilder,
) {
    builder.apply_n0();
}

#[ffi_export]
fn iroh_builder_apply_minimal(
    builder: &EndpointBuilder,
) {
    builder.apply_minimal();
}

#[ffi_export]
fn iroh_builder_secret_key(
    builder: &EndpointBuilder,
    key: repr_c::Vec<u8>,
) -> bool {
    let bytes : Vec<u8> = key.into();
    builder.secret_key(bytes).is_ok()
}

#[ffi_export]
fn iroh_builder_add_alpn(
    builder: &EndpointBuilder,
    alpn: char_p::Ref<'_>,
) -> bool {
    builder.alpns(vec![
        alpn.to_str().as_bytes().to_vec(),
    ]);
    true
}

#[ffi_export]
fn iroh_builder_relay_mode(
    builder: &EndpointBuilder,
    mode: u8,
) {
    match mode {
        0 => builder.relay_mode(&RelayMode::Default),
        1 => builder.relay_mode(&RelayMode::Disabled),
        _ => builder.relay_mode(&RelayMode::Default),
    }
}

#[ffi_export]
fn iroh_builder_bind_addr(
    builder: &EndpointBuilder,
    addr: char_p::Ref<'_>,
) -> bool {
    builder.bind_addr(addr.to_string()).is_ok()
}


#[cfg(feature = "headers")]
#[test]
fn generate_headers() -> std::io::Result<()> {
    safer_ffi::headers::builder()
        .to_file("include/iroh.h")?
        .generate()
}


