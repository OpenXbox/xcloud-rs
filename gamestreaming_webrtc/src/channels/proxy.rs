use crate::GssvChannelEvent;

use super::{
    base::{
        ChannelExchangeMsg, ChannelType, DataChannelMsg, DataChannelParams, GssvChannel,
        GssvChannelProperties,
    },
    chat::ChatChannel,
    control::ControlChannel,
    input::InputChannel,
    message::MessageChannel,
};
use tokio::sync::mpsc;

#[derive(Debug)]
pub struct ChannelProxy {
    input: InputChannel,
    control: ControlChannel,
    message: MessageChannel,
    chat: ChatChannel,
    channel_to_client_mpsc: (
        mpsc::Sender<(ChannelType, ChannelExchangeMsg)>,
        mpsc::Receiver<(ChannelType, ChannelExchangeMsg)>,
    ),
}

impl ChannelProxy {
    pub fn data_channel_create_params() -> &'static [(ChannelType, DataChannelParams)] {
        &[
            (ChannelType::Input, InputChannel::PARAMS),
            (ChannelType::Control, ControlChannel::PARAMS),
            (ChannelType::Message, MessageChannel::PARAMS),
            (ChannelType::Chat, ChatChannel::PARAMS),
        ]
    }

    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(10);

        Self {
            input: InputChannel::new(tx.clone()),
            control: ControlChannel::new(tx.clone()),
            message: MessageChannel::new(tx.clone()),
            chat: ChatChannel::new(tx.clone()),
            channel_to_client_mpsc: (tx, rx),
        }
    }

    /// Used to receive messages from ChannelProxy in client
    pub fn get_receiver(self) -> mpsc::Receiver<(ChannelType, ChannelExchangeMsg)> {
        self.channel_to_client_mpsc.1
    }

    pub async fn handle_event(
        &mut self,
        typ: ChannelType,
        event: GssvChannelEvent,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match typ {
            ChannelType::Input => {
                let channel = &self.input;
                match event {
                    GssvChannelEvent::ChannelOpen => channel.on_open().await,
                    GssvChannelEvent::ChannelClose => channel.on_close().await,
                }
            },
            ChannelType::Control => {
                let channel = &self.control;
                match event {
                    GssvChannelEvent::ChannelOpen => channel.on_open().await,
                    GssvChannelEvent::ChannelClose => channel.on_close().await,
                }
            },
            ChannelType::Message => {
                let channel = &self.message;
                match event {
                    GssvChannelEvent::ChannelOpen => channel.on_open().await,
                    GssvChannelEvent::ChannelClose => channel.on_close().await,
                }
            },
            ChannelType::Chat => {
                let channel = &self.chat;
                match event {
                    GssvChannelEvent::ChannelOpen => channel.on_open().await,
                    GssvChannelEvent::ChannelClose => channel.on_close().await,
                }
            },
            _ => {
                return Err(format!("Unhandled channel type {:?}", typ).into());
            },
        }
    }

    pub async fn handle_message(
        &mut self,
        typ: ChannelType,
        msg: DataChannelMsg,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match typ {
            ChannelType::Input => self.input.on_message(&msg).await,
            ChannelType::Control => self.control.on_message(&msg).await,
            ChannelType::Message => self.message.on_message(&msg).await,
            ChannelType::Chat => self.chat.on_message(&msg).await,
            _ => {
                return Err(format!("Unhandled channel type {:?}", typ).into());
            },
        }
    }
}

impl Default for ChannelProxy {
    fn default() -> Self {
        Self::new()
    }
}
