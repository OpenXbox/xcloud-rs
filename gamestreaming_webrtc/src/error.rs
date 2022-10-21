use thiserror::Error;

use crate::api::GssvApiError;

#[derive(Error, Debug)]
pub enum PacketError {
    #[error("Unknown error")]
    Unknown,
}

#[derive(Error, Debug)]
pub enum GsError {
    #[error("Invalid platform provided")]
    InvalidPlatform(String),
    #[error(transparent)]
    ApiError(#[from] GssvApiError),
    #[error("Connection provisioning failed")]
    Provisioning(String),
    #[error("Connection exchange failed")]
    ConnectionExchange(String),
    #[error("Unknown error")]
    Unknown,
}
