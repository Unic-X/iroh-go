use std::fmt;

#[derive(Debug)]
pub enum IrohError {
    EndpointNotFound,
    EndpointBuilderConsumed,
    InvalidNodeId(String),
    Iroh(String),
    InvalidSecretKeyLength(usize),
    InvalidSocketAddr(String),
    BindAddrError(String),
}

impl fmt::Display for IrohError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IrohError::EndpointNotFound => write!(f, "endpoint not found"),
            IrohError::InvalidNodeId(e) => write!(f, "invalid node id: {}", e),
            IrohError::Iroh(e) => write!(f, "iroh error: {}", e),
            IrohError::EndpointBuilderConsumed => write!(f, "endpoint builder consumed"),
            IrohError::InvalidSecretKeyLength(len) => write!(f, "invalid secret key length: {}", len),
            IrohError::InvalidSocketAddr(e) => write!(f, "invalid socket addr: {}", e),
            IrohError::BindAddrError(e) => write!(f, "bind addr error: {}", e),
        }
    }
}

impl std::error::Error for IrohError {}

macro_rules! from_iroh_err {
    ($($err:ty),* $(,)?) => {
        $(
            impl From<$err> for IrohError {
                fn from(err: $err) -> Self {
                    IrohError::Iroh(err.to_string())
                }
            }
        )*
    };
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
    iroh_tickets::ParseError,
}