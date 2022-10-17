#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AudioPacketType {
    ServerHandshake = 1,
    ClientHandshake = 2,
    Control = 3,
    Data = 4,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AudioCodec {
    Opus = 0,
    Pcm = 1,
    Aac = 2,
}

bitflags! {
    pub struct AudioControlFlags : u32 {
        const STOP_STREAM = 0x08;
        const START_STREAM = 0x10;
        const REINITIALIZE = 0x40;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AudioDataFlags {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PCMAudioFormat {
    pub bits: u32,
    pub is_float: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioFormat {
    pub channels: u32,
    pub frequency: u32,
    pub codec: u32,
    pub pcm_format: Option<PCMAudioFormat>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioServerHandshake {
    pub protocol_version: u32,
    pub reference_timestamp: u64,
    pub format_count: u32,
    pub formats: Box<[AudioFormat]>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioClientHandshake {
    pub initial_frame_id: u32,
    pub requested_format: AudioFormat,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioControl {
    pub flags: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioData {
    pub flags: u32,
    pub frame_id: u32,
    pub timestamp: u64,
    pub data_size: u32,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AudioPacket {
    ServerHandshake(AudioServerHandshake),
    ClientHandshake(AudioClientHandshake),
    Control(AudioControl),
    Data(AudioData),
}
