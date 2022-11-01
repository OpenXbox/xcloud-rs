pub mod api;
mod channels;
mod client;
pub mod error;
mod packets;
mod serde_helpers;

pub use channels::{
    base::{ChannelType, DataChannelParams, DataChannelMsg, ChannelExchangeMsg, GssvChannel, GssvChannelEvent, GssvChannelProperties},
    proxy::ChannelProxy,
};
pub use client::{GamestreamingClient, Platform};
