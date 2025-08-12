use std::sync::Arc;

use webrtc::data_channel::data_channel_message::DataChannelMessage;

use crate::error::GsError;

use super::base::GssvChannel;

pub struct ChatChannel {
    conn: Arc<webrtc::peer_connection::RTCPeerConnection>,
    inner: Arc<webrtc::data_channel::RTCDataChannel>,
}

impl GssvChannel for ChatChannel {
    fn id() -> i32 {
        6
    }

    fn protocol() -> &'static str {
        "chatV1"
    }

    fn is_ordered() -> Option<bool> {
        None
    }

    fn name() -> &'static str {
        "chat"
    }

    fn new(peer_connection: Arc<webrtc::peer_connection::RTCPeerConnection>, inner: Arc<webrtc::data_channel::RTCDataChannel>) -> Self {
        Self {
            conn: peer_connection,
            inner,
        }
    }

    fn conn(&self) -> Arc<webrtc::peer_connection::RTCPeerConnection> {
        self.conn.clone()
    }

    fn datachannel(&self) -> Arc<webrtc::data_channel::RTCDataChannel> {
        self.inner.clone()
    }

    async fn on_open(self: Arc<Self>) {
        log::warn!("TODO: Implement on_open for channel: '{}'", Self::name());
    }

    async fn on_close(self: Arc<Self>) {
        log::warn!("TODO: Implement on_close for channel: '{}'", Self::name());
    }

    async fn start(&self) -> Result<(), GsError> {
        log::warn!("TODO: Implement start for channel: '{}'", Self::name());
        Ok(())
    }
    
    async fn on_message(self: Arc<Self>, msg: DataChannelMessage) {
        log::warn!("on_message (channel: {}): {:?}", Self::name(), msg);
        log::warn!("TODO: Implement on_message for channel: '{}'", Self::name());
    }
    
    async fn on_error(self: Arc<Self>, error: webrtc::Error) {
        log::error!("Datachannel error, channel: {}, error: {error}", Self::name())
    }
}
