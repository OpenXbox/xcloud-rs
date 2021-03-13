#[derive(Debug, Clone, PartialEq)]
pub enum MessagePacketType {
    Handshake = 1,
    Data = 2,
    CancelRequest = 3,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MessageData {
    pub unknown1: u32,
    pub unknown2: u32,
    pub unknown3: u32,
    pub unknown4: u32,
    pub unknown5: u32,
    pub unknown6: u32,
}
