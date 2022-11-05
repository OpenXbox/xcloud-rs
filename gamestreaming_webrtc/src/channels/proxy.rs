use crate::{GssvChannelEvent, GamepadData};

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
    channel_to_client_mpsc: mpsc::Sender<(ChannelType, ChannelExchangeMsg)>,
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

    pub fn new(sender: mpsc::Sender<(ChannelType, ChannelExchangeMsg)>) -> Self {
        Self {
            input: InputChannel::new(sender.clone()),
            control: ControlChannel::new(sender.clone()),
            message: MessageChannel::new(sender.clone()),
            chat: ChatChannel::new(sender.clone()),
            channel_to_client_mpsc: sender,
        }
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
            ChannelType::Message => {
                // Start control / input channel on HandshakeAck @ message-channel
                if let DataChannelMsg::String(msg) = &msg {
                    let msg: Result<serde_json::Value, serde_json::Error> = serde_json::from_str(msg);
                    if let Ok(deserialized) = msg {
                        if let Some(typ) = deserialized.get("Type") {
                            if typ.is_string() && typ.as_str().unwrap() == "HandshakeAck" {
                                self.input.start().await?;
                                self.control.start().await?;
                            }
                        }
                    }
                }

                self.message.on_message(&msg).await
            },
            ChannelType::Chat => self.chat.on_message(&msg).await,
            _ => {
                return Err(format!("Unhandled channel type {:?}", typ).into());
            },
        }
    }

    pub async fn handle_input(&mut self, data: &GamepadData) -> Result<(), Box<dyn std::error::Error>> {
        self.input.on_button_press(data).await
    }
}
