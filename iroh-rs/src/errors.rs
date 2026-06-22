use std::fmt;

use safer_ffi::{
    derive_ReprC,
    prelude::*,
};

#[derive_ReprC]
#[repr(C)]
#[derive(Debug, Clone)]
pub struct IrohError {
    pub message: repr_c::String,
}

macro_rules! from_iroh_err {
    ($($path:path),* $(,)?) => {
        $(
            impl From<$path> for IrohError {
                fn from(value: $path) -> Self {
                    Self {
                        message: format!("{:?}", value).into(),
                    }
                }
            }
        )*
    };
}

impl From<anyhow::Error> for IrohError {
    fn from(e: anyhow::Error) -> Self {
        Self {
            message: e.to_string().into(),
        }
    }
}

impl fmt::Display for IrohError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for IrohError {}

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
    n0_future::task::JoinError,
}

#[derive(Debug, PartialEq, Eq)]
pub enum CallbackError {
    Error,
}

impl From<anyhow::Error> for CallbackError {
    fn from(_e: anyhow::Error) -> Self {
        CallbackError::Error
    }
}


#[derive_ReprC]
#[repr(u8)]
pub enum IrohResultTag {
    Ok,
    Error,
}

#[derive_ReprC]
#[repr(C)]
pub struct IrohResult<T>
where
    T: ReprC,
{
    pub tag: IrohResultTag,
    pub value: repr_c::TaggedOption<T>,
    pub error: repr_c::TaggedOption<IrohError>,
}

impl<T> IrohResult<T>
where
    T: ReprC,
{
    pub fn ok(value: T) -> Self {
        Self {
            tag: IrohResultTag::Ok,
            value: Some(value).into(),
            error: None.into(),
        }
    }

    pub fn err(error: IrohError) -> Self {
        Self {
            tag: IrohResultTag::Error,
            value: None.into(),
            error: Some(error).into(),
        }
    }
}

impl<T> IrohResult<T>
where
    T: ReprC,
{
    pub fn from_result<E>(r: Result<T, E>) -> Self
    where
        E: Into<IrohError>,
    {
        match r {
            Ok(v) => Self::ok(v),
            Err(e) => Self::err(e.into()),
        }
    }
}

impl<T: ReprC> IrohResult<T> {
    pub fn unwrap(self) -> T {
        match self.tag {
            IrohResultTag::Ok => self.value.into_rust().unwrap(),
            IrohResultTag::Error => {
                panic!("{:?}", self.error.into_rust().unwrap())
            }
        }
    }
}