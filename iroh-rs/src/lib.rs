mod errors;
mod endpoint;
mod runtime;
mod key;
mod net;
mod ticket;
mod accept;
mod connection;
mod relay;
mod path;
mod watch;


pub use self::{
    accept::*, endpoint::*, errors::*, key::*, net::*, ticket::*, connection::*, relay::*,path::*, runtime::*
};


#[cfg(feature = "headers")]
#[test]
fn generate_headers() -> std::io::Result<()> {
    safer_ffi::headers::builder()
        .to_file("include/iroh.h")?
        .generate()
}


