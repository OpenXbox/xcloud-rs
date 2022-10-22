use deku::prelude::*;

#[derive(Debug, Clone, DekuRead, DekuWrite, PartialEq, Eq)]
#[deku(type = "u32")]
pub enum AudioPacketType {
    ServerHandshake = 1,
    ClientHandshake = 2,
    Control = 3,
    Data = 4,
}

#[derive(Debug, Clone, DekuRead, DekuWrite, PartialEq, Eq)]
#[deku(type = "u32")]
pub enum AudioCodec {
    Opus = 0,
    Pcm = 1,
    Aac = 2,
}

/// Audio control flags either commands the host to start, stop
/// or reinit the audio stream.
/// Total bits: 32
#[derive(Debug, Clone, DekuRead, DekuWrite, Eq, PartialEq, Default)]
pub struct AudioControlFlags {
    /// Reinit audio stream
    /// Bit 30 / Mask LE 0x40000000 BE 0x40
    #[deku(pad_bits_before = "1", bits = "1")]
    reinitialize: bool,

    /// Start audio stream
    /// Bit 28 / Mask LE 0x10000000 BE 0x10
    #[deku(pad_bits_before = "1", bits = "1")]
    start_stream: bool,

    /// Stop audio stream
    /// Bit 27 / Mask LE 0x08000000 BE 0x08
    // Pad to end of 32 bits
    #[deku(pad_bits_after = "26", bits = "1")]
    stop_stream: bool,
}

#[derive(Debug, Clone, DekuRead, DekuWrite, PartialEq, Eq)]
pub struct AudioDataFlags {
    // TODO: Found out what these are
    pub unknown: u32,
}

#[derive(Debug, Clone, DekuRead, DekuWrite, PartialEq, Eq)]
pub struct PCMAudioFormat {
    pub bits: u32,
    pub is_float: u32,
}

#[derive(Debug, Clone, DekuRead, DekuWrite, PartialEq, Eq)]
pub struct AudioFormat {
    pub channels: u32,
    pub frequency: u32,
    pub codec: AudioCodec,
    #[deku(cond = "*codec == AudioCodec::Pcm")]
    pub pcm_format: Option<PCMAudioFormat>,
}

#[derive(Debug, Clone, DekuRead, DekuWrite, PartialEq, Eq)]
pub struct AudioServerHandshake {
    pub protocol_version: u32,
    pub reference_timestamp: u64,
    #[deku(update = "self.formats.len()")]
    pub format_count: u32,
    #[deku(count = "format_count")]
    pub formats: Vec<AudioFormat>,
}

#[derive(Debug, Clone, DekuRead, DekuWrite, PartialEq, Eq)]
pub struct AudioClientHandshake {
    pub initial_frame_id: u32,
    pub requested_format: AudioFormat,
}

#[derive(Debug, Clone, DekuRead, DekuWrite, PartialEq, Eq)]
pub struct AudioControl {
    pub flags: AudioControlFlags,
}

#[derive(Debug, Clone, DekuRead, DekuWrite, PartialEq, Eq)]
pub struct AudioData {
    pub flags: AudioDataFlags,
    pub frame_id: u32,
    pub timestamp: u64,
    #[deku(update = "self.data.len()")]
    pub data_size: u32,
    #[deku(count = "data_size")]
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
    use std::default::Default;

    use super::*;

    #[test]
    fn parse_audio_control_flags() {
        fn create_flag(val: [u8; 4]) -> AudioControlFlags {
            let (_, flags) =
                AudioControlFlags::from_bytes((&val, 0)).expect("Failed to create flags");

            flags
        }
        let start_flag = create_flag([0x10, 0, 0, 0]);
        let stop_flag = create_flag([0x08, 0, 0, 0]);
        let reinit_flag = create_flag([0x40, 0, 0, 0]);
        let start_reinit_flag = create_flag([0x50, 0, 0, 0]);

        assert!(start_flag.start_stream);
        assert!(!start_flag.stop_stream);
        assert!(!start_flag.reinitialize);

        assert!(!stop_flag.start_stream);
        assert!(stop_flag.stop_stream);
        assert!(!stop_flag.reinitialize);

        assert!(!reinit_flag.start_stream);
        assert!(!reinit_flag.stop_stream);
        assert!(reinit_flag.reinitialize);

        assert!(start_reinit_flag.start_stream);
        assert!(!start_reinit_flag.stop_stream);
        assert!(start_reinit_flag.reinitialize);
    }

    #[test]
    fn serialize_audio_control_flags() {
        let start_flag = AudioControlFlags {
            start_stream: true,
            ..Default::default()
        };
        let stop_flag = AudioControlFlags {
            stop_stream: true,
            ..Default::default()
        };
        let reinit_flag = AudioControlFlags {
            reinitialize: true,
            ..Default::default()
        };
        let start_reinit_flag = AudioControlFlags {
            start_stream: true,
            reinitialize: true,
            ..Default::default()
        };

        assert_eq!(start_flag.to_bytes().unwrap(), vec![0x10, 0, 0, 0]);
        assert_eq!(stop_flag.to_bytes().unwrap(), vec![0x08, 0, 0, 0]);
        assert_eq!(reinit_flag.to_bytes().unwrap(), vec![0x40, 0, 0, 0]);
        assert_eq!(start_reinit_flag.to_bytes().unwrap(), vec![0x50, 0, 0, 0]);
    }
}
