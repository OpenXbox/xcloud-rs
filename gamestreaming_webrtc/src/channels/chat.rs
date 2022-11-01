use async_trait::async_trait;
use tokio::sync::mpsc;

use super::base::{
    ChannelExchangeMsg, ChannelType, DataChannelParams, GssvChannel, GssvChannelProperties,
};

#[derive(Debug)]
pub struct ChatChannel {
    sender: mpsc::Sender<(ChannelType, ChannelExchangeMsg)>,
}

impl ChatChannel {
    pub fn new(sender: mpsc::Sender<(ChannelType, ChannelExchangeMsg)>) -> Self {
        Self { sender }
    }
}

impl GssvChannelProperties for ChatChannel {
    const TYPE: ChannelType = ChannelType::Chat;
    const PARAMS: DataChannelParams = DataChannelParams {
        id: 6,
        protocol: "chatV1",
        is_ordered: None,
    };
    fn sender(&self) -> &mpsc::Sender<(ChannelType, ChannelExchangeMsg)> {
        &self.sender
    }
}

#[async_trait]
impl GssvChannel for ChatChannel {}
