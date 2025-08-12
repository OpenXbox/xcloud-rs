use std::sync::Arc;

use crate::error::GsError;

use super::base::{GssvChannel, GssvChannelSend};
use serde_json::json;
use webrtc::data_channel::data_channel_message::DataChannelMessage;

pub struct ControlChannel {
    conn: Arc<webrtc::peer_connection::RTCPeerConnection>,
    inner: Arc<webrtc::data_channel::RTCDataChannel>,
}

impl GssvChannel for ControlChannel {
    fn id() -> i32 {
        4
    }

    fn protocol() -> &'static str {
        "controlV1"
    }

    fn is_ordered() -> Option<bool> {
        None
    }

    fn name() -> &'static str {
        "control"
    }

    fn new(peer_connection: Arc<webrtc::peer_connection::RTCPeerConnection>, inner: Arc<webrtc::data_channel::RTCDataChannel>) -> Self {
        Self {
            conn: peer_connection,
            inner: inner,
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
        let auth_request = json!({
            "message":"authorizationRequest",
            "accessKey":"4BDB3609-C1F1-4195-9B37-FEFF45DA8B8E",
        });
        self.send_message(&auth_request.into()).await?;

        let gamepad_request = json!({
            "message": "gamepadChanged",
            "gamepadIndex": 0,
            "wasAdded": true,
        });
        self.send_message(&gamepad_request.into()).await
    }

    async fn on_message(self: Arc<Self>, msg: DataChannelMessage) {
        log::warn!("on_message (channel: {}): {:?}", Self::name(), msg);
        log::warn!("TODO: Implement on_message for channel: '{}'", Self::name());
    }

    async fn on_error(self: Arc<Self>, error: webrtc::Error) {
        log::error!("Datachannel error, channel: {}, error: {error}", Self::name())
    }
}

impl ControlChannel {
    async fn request_keyframe(&self) -> Result<(), GsError> {
        let keyframe_request = json!({
            "message": "videoKeyframeRequested",
            "ifrRequested": true,
        });

        self.send_message(&keyframe_request.into()).await
    }
}
