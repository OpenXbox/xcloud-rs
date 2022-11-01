use super::base::{
    ChannelExchangeMsg, ChannelType, DataChannelParams, GssvChannel, GssvChannelProperties,
};
use async_trait::async_trait;
use serde_json::json;
use tokio::sync::mpsc;

#[derive(Debug)]
pub struct ControlChannel {
    sender: mpsc::Sender<(ChannelType, ChannelExchangeMsg)>,
}

impl ControlChannel {
    pub fn new(sender: mpsc::Sender<(ChannelType, ChannelExchangeMsg)>) -> Self {
        Self { sender }
    }

    pub(crate) async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        let auth_request = json!({
            "message":"authorizationRequest",
            "accessKey":"4BDB3609-C1F1-4195-9B37-FEFF45DA8B8E",
        });
        self.send_message(auth_request.into()).await?;

        let gamepad_request = json!({
            "message": "gamepadChanged",
            "gamepadIndex": 0,
            "wasAdded": true,
        });
        self.send_message(gamepad_request.into()).await
    }

    async fn request_keyframe(&self) -> Result<(), Box<dyn std::error::Error>> {
        let keyframe_request = json!({
            "message": "videoKeyframeRequested",
            "ifrRequested": true,
        });

        self.send_message(keyframe_request.into()).await
    }
}

impl GssvChannelProperties for ControlChannel {
    const TYPE: ChannelType = ChannelType::Control;
    const PARAMS: DataChannelParams = DataChannelParams {
        id: 4,
        protocol: "controlV1",
        is_ordered: None,
    };
    fn sender(&self) -> &mpsc::Sender<(ChannelType, ChannelExchangeMsg)> {
        &self.sender
    }
}

#[async_trait]
impl GssvChannel for ControlChannel {}
