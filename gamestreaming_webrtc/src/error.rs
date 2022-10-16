use enumflags2::{BitFlag, FromBitsError};

use thiserror::Error;

use crate::api::GssvApiError;

#[derive(Error, Debug)]
pub enum PacketError {
    #[error("Unknown error")]
    Unknown,
}

#[derive(Error, Debug)]
pub enum FlagsError<T>
where
    T: BitFlag,
{
    #[error(transparent)]
    DeserializeError(#[from] FromBitsError<T>),
}

#[derive(Error, Debug)]
pub enum GsError {
    #[error(transparent)]
    ApiError(#[from] GssvApiError),
    #[error("Unknown error")]
    Unknown,
}
