pub mod api;
pub mod channels;
mod client;
pub mod error;
mod packets;
mod serde_helpers;

// Re-export webrtc and auth
pub use webrtc;
pub use gamestreaming_auth as auth;

pub use client::{GamestreamingClient, Platform};
