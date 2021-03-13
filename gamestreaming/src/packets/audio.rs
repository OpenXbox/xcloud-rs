#[derive(Debug, Clone, PartialEq)]
pub enum AudioPacketType {
    ServerHandshake = 1,
    ClientHandshake = 2,
    Control = 3,
    Data = 4,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AudioCodec {
    Opus = 0,
    PCM = 1,
    AAC = 2
}

#[derive(Debug, Clone, PartialEq)]
pub enum AudioControlFlags {
    StopStream = 0x08,
    StartStream = 0x10,
    Reinitialize = 0x40,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PCMAudioFormat {
    pub bits: u32,
    pub is_float: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AudioFormat {
    pub channels: u32,
    pub frequency: u32,
    pub codec: u32,
    pub pcm_format: Option<PCMAudioFormat>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AudioServerHandshake {
    pub protocol_version: u32,
    pub reference_timestamp: u64,
    pub format_count: u32,
    pub formats: Box<[AudioFormat]>
}

#[derive(Debug, Clone, PartialEq)]
pub struct AudioClientHandshake {
    pub initial_frame_id: u32,
    pub requested_format: AudioFormat
}

#[derive(Debug, Clone, PartialEq)]
pub struct AudioControl {
    pub flags: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AudioData {
    pub flags: u32,
    pub frame_id: u32,
    pub timestamp: u64,
    pub data_size: u32,
    pub data: Vec<u8>
}

#[derive(Debug, Clone, PartialEq)]
pub enum AudioPacket {
    ServerHandshake(AudioServerHandshake),
    ClientHandshake(AudioClientHandshake),
    Control(AudioControl),
    Data(AudioData)
}