pub mod api;
mod channels;
mod client;
pub mod error;
mod packets;
mod serde_helpers;
#[cfg(feature="gamepad")]
mod gamepad;
#[cfg(feature="gamepad")]
pub use gamepad::GamepadProcessor;

pub use channels::{
    base::{
        ChannelType, DataChannelParams, DataChannelMsg, ChannelExchangeMsg, GssvChannel,
        GssvClientEvent,GssvChannelEvent, GssvChannelProperties
    },
    proxy::ChannelProxy,
};

pub use packets::input::GamepadData;
pub use client::{GamestreamingClient, Platform};
