#[derive(Debug, Clone, PartialEq)]
pub enum InputPacketType {
    ServerHandshakeV3 = 1,
    ClientHandshakeV3 = 2,
    FrameAck = 3,
    FrameV3 = 4,

    ServerHandshakeV4 = 5,
    ClientHandshakeV4 = 6,
    FrameV4 = 7,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InputServerHandshake {
    pub min_protocol_version: u32,
    pub max_protocol_version: u32,
    pub desktop_width: u32,
    pub desktop_height: u32,
    pub maximum_touches: u32,
    pub initial_frame_id: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InputClientHandshake {
    pub min_protocol_version: u32,
    pub max_protocol_version: u32,
    pub maximum_touches: u32,
    pub reference_timestamp: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InputPacket {
    ServerHandshake(InputServerHandshake),
    ClientHandshake(InputClientHandshake),
    FrameAck,
    Frame
}