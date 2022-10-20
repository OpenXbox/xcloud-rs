#[derive(Debug)]
pub enum ChannelType {
    Chat,
    Control,
    Input,
    Message,
    Audio,
    Video,
}

#[derive(Debug)]
pub struct GssvChannelEvent(String);

#[derive(Debug)]
pub enum DataChannelMsg {
    String(String),
    Bytes(Vec<u8>),
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

pub trait GssvChannel {
    fn name() -> &'static str;
    fn on_open(&self);
    fn on_close(&self);
    fn start(&mut self) {
        todo!("Channel start not implemented")
    }
    fn on_message(&self, msg: &DataChannelMsg) -> Result<(), Box<dyn std::error::Error>> {
        println!("on_message ({}): {:?}", Self::name(), msg);
        todo!()
    }
    fn send_message(&self, msg: &DataChannelMsg);
    fn send_event(&self, event: &GssvChannelEvent);
}
