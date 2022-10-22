pub mod api;
mod channels;
mod client;
pub mod error;
mod packets;
mod serde_helpers;

// Re-export auth
pub use gamestreaming_auth as auth;

pub use client::{GamestreamingClient, Platform};
