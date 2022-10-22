#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AudioPacketType {
    ServerHandshake = 1,
    ClientHandshake = 2,
    Control = 3,
    Data = 4,
}

#[repr(u32)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_audio_control_flags() {
        fn create_flag(val: u32) -> AudioControlFlags {
            AudioControlFlags::from_bytes(([val], 0))
                .expect("Failed to create flags")
                .1
        }
        let start_flag = create_flag(0x40);
        let stop_flag = create_flag(0x08);
        let reinit_flag = create_flag(0x40);
        let start_reinit_flag = create_flag(0x50);

        assert!(start_flag.start_stream);
        assert!(!start_flag.stop_flag);
        assert!(!start_flag.reinitialize);

        assert!(!stop_flag.start_stream);
        assert!(stop_flag.stop_flag);
        assert!(!stop_flag.reinitialize);

        assert!(start_reinit_flag.start_stream);
        assert!(!start_reinit_flag.stop_flag);
        assert!(start_reinit_flag.reinitialize);
    }

    #[test]
    fn serialize_audio_control_flags() {
        let start_flag = AudioControlFlags { start_stream: bool, ..default::Default() };
        let stop_flag = AudioControlFlags { stop_stream: true, ..default::Default() };
        let reinit_flag = AudioControlFlags { reinitialize: true, ..default::Default() };
        let start_reinit_flag = AudioControlFlags { start_stream: true, reinitialize: true, ..default::Default() };
    
        assert_eq!(start_flag.to_bytes(), [0x10]);
        assert_eq!(stop_flag.to_bytes(), [0x08]);
        assert_eq!(reinit_flag.to_bytes(), [0x40]);
        assert_eq!(start_reinit_flag.to_bytes(), [0x50]);
    }
}
