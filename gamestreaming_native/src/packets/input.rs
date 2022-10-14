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
pub struct InputFrameAck {
    pub acked_frame_id: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InputFrameV3 {
    pub frame_id: u32,
    pub timestamp: i64,
    pub frame: FrameV3Data,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FrameV3Data {
    pub input_count: u32,
    pub unknown: Option<Vec<(u32, u32, u32)>>, // length: input_count
    pub data_mouse: Option<MouseData>,
    pub data_gamepad: Option<GamepadData>,
    pub data_keyboard: Option<KeyboardData>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MouseData {}

#[derive(Debug, Clone, PartialEq)]
pub struct GamepadData {}

#[derive(Debug, Clone, PartialEq)]
pub struct KeyboardData {}

#[derive(Debug, Clone, PartialEq)]
pub struct InputFrameV4 {
    pub frame_id: u32,
    pub timestamp: i64,
    pub frame_changes: FrameChanges,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FrameChanges {}

#[derive(Debug, Clone, PartialEq)]
pub enum InputPacket {
    ServerHandshake(InputServerHandshake),
    ClientHandshake(InputClientHandshake),
    FrameAck,
    Frame,
}
