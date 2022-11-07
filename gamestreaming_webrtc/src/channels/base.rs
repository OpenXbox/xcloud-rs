use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::packets::input::VibrationReport;

#[derive(Debug, Eq, PartialEq, Hash, Clone, Copy)]
pub enum ChannelType {
    Chat,
    Control,
    Input,
    Message,
    Audio,
    Video,
}

impl ToString for ChannelType {
    fn to_string(&self) -> String {
        let res = match self {
            ChannelType::Chat => "chat",
            ChannelType::Control => "control",
            ChannelType::Input => "input",
            ChannelType::Message => "message",
            ChannelType::Audio => "audio",
            ChannelType::Video => "video",
        };
        res.to_owned()
    }
}

#[derive(Debug)]
pub enum GssvChannelEvent {
    /// Controller Rumble (Channels to Client)
    GamepadRumble(VibrationReport),
}

#[derive(Debug)]
pub enum GssvClientEvent {
    /// Data Channel opened (Client to Channels)
    ChannelOpen,
    /// Data Channel closed (Client to Channels)
    ChannelClose,
}

#[derive(Debug)]
pub enum DataChannelMsg {
    String(String),
    Bytes(Vec<u8>),
}

#[derive(Debug)]
pub enum ChannelExchangeMsg {
    ChannelEvent(GssvChannelEvent),
    ClientEvent(GssvClientEvent),
    DataChannel(DataChannelMsg),
}

impl From<serde_json::Value> for DataChannelMsg {
    fn from(val: serde_json::Value) -> Self {
        let str =
            serde_json::to_string(&val).expect("Failed to serialize message for DataChannelMsg");
        DataChannelMsg::String(str)
    }
}

impl TryFrom<&DataChannelMsg> for serde_json::Value {
    type Error = Box<dyn std::error::Error>;

    fn try_from(value: &DataChannelMsg) -> Result<Self, Self::Error> {
        match value {
            DataChannelMsg::String(str) => serde_json::from_str(str).map_err(|e| e.into()),
            _ => Err("Can only convert DataChannelMsg::String to JSON".into()),
        }
    }
}

#[derive(Debug, Default, Clone, Copy, Eq, PartialEq)]
pub struct DataChannelParams {
    pub id: i32,
    pub protocol: &'static str,
    pub is_ordered: Option<bool>,
}

pub trait GssvChannelProperties {
    const TYPE: ChannelType;
    const PARAMS: DataChannelParams;
    fn sender(&self) -> &mpsc::Sender<(ChannelType, ChannelExchangeMsg)>;
}

#[async_trait]
pub trait GssvChannel
where
    Self: GssvChannelProperties,
{
    async fn on_open(&self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    async fn on_close(&self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    async fn on_message(&self, msg: &DataChannelMsg) -> Result<(), Box<dyn std::error::Error>> {
        let msg = format!("Unhandled on_message ({:?}): {:?}", Self::TYPE, msg);
        Err(msg.into())
    }

    async fn send_message(&self, msg: DataChannelMsg) -> Result<(), Box<dyn std::error::Error>> {
        //println!("Send message: {:?}", &msg);
        self.sender()
            .send((Self::TYPE, ChannelExchangeMsg::DataChannel(msg)))
            .await
            .map_err(|e| e.into())
    }

    async fn send_event(&self, event: GssvChannelEvent) -> Result<(), Box<dyn std::error::Error>> {
        self.sender()
            .send((Self::TYPE, ChannelExchangeMsg::ChannelEvent(event)))
            .await
            .map_err(|e| e.into())
    }
}
