pub mod api;
mod channels;
mod client;
pub mod error;
mod packets;
mod serde_helpers;

pub use client::{GamestreamingClient, Platform};
