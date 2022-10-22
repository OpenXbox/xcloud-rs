use deku::prelude::*;

#[derive(Debug, Clone, DekuRead, DekuWrite, PartialEq, Eq)]
#[deku(type = "u32")]
pub enum MessagePacketType {
    Handshake = 1,
    Data = 2,
    CancelRequest = 3,
}

#[derive(Debug, Clone, DekuRead, DekuWrite, PartialEq, Eq)]
pub struct MessageHandshake {
    pub unknown: u32,
}

#[derive(Debug, Clone, DekuRead, DekuWrite, PartialEq, Eq)]
pub struct MessageData {
    pub unknown1: u32,
    pub unknown2: u32,
    pub unknown3: u32,
    pub unknown4: u32,
    pub unknown5: u32,
    pub unknown6: u32,
}

#[derive(Debug, Clone, DekuRead, DekuWrite, PartialEq, Eq)]
pub struct MessageCancelRequest {
    pub unknown: u32,
}
