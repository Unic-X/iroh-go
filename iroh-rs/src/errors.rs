use std::fmt;

#[derive(Debug)]
pub enum IrohError {
    EndpointNotFound,
    InvalidNodeId(String),
    Iroh(String),
}

impl fmt::Display for IrohError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IrohError::EndpointNotFound => write!(f, "endpoint not found"),
            IrohError::InvalidNodeId(e) => write!(f, "invalid node id: {}", e),
            IrohError::Iroh(e) => write!(f, "iroh error: {}", e),
        }
    }
}

impl std::error::Error for IrohError {}

impl From<iroh::endpoint::BindError> for IrohError {
    fn from(error: iroh::endpoint::BindError) -> Self {
        IrohError::Iroh(error.to_string())
    }
}

impl From<iroh::endpoint::ConnectError> for IrohError {
    fn from(error: iroh::endpoint::ConnectError) -> Self {
        IrohError::Iroh(error.to_string())
    }
}
