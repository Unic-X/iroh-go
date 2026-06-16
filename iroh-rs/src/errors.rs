use std::fmt;
use std::error::Error;
use safer_ffi::derive_ReprC;

#[derive_ReprC]
#[repr(opaque)]
#[derive(Debug)]
pub struct IrohError {
    e: anyhow::Error,
}

macro_rules! from_iroh_err {
    ($($path:path),* $(,)?) => {
        $(
            impl From<$path> for IrohError {
                fn from(value: $path) -> Self {
                    Self {
                        e: anyhow::anyhow!("{:?}", value),
                    }
                }
            }
        )*
    };
}

impl From<anyhow::Error> for IrohError {
    fn from(e: anyhow::Error) -> Self {
        Self { e }
    }
}

impl fmt::Display for IrohError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.e)
    }
}



from_iroh_err! {
    iroh::endpoint::BindError,
    iroh::endpoint::ConnectError,
    iroh::endpoint::ConnectionError,
    iroh::endpoint::AlpnError,
    iroh::endpoint::RemoteEndpointIdError,
    iroh::endpoint::VarIntBoundsExceeded,
    iroh::endpoint::WriteError,
    iroh::endpoint::ClosedStream,
    iroh::endpoint::ReadError,
    iroh::endpoint::ReadExactError,
    iroh::endpoint::ReadToEndError,
    iroh::endpoint::StoppedError,
    iroh::endpoint::SendDatagramError,
    iroh::endpoint::ResetError,
    iroh_base::KeyParsingError,
    iroh_tickets::ParseError,
}