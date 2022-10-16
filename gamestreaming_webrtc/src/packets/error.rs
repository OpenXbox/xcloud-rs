use enumflags2::{BitFlag, FromBitsError};

use thiserror::Error;

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
