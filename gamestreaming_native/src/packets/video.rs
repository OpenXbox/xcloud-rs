use deku::prelude::*;

#[derive(Debug, Clone, DekuRead, DekuWrite, PartialEq, Eq)]
#[deku(type = "u32")]
pub enum VideoPacketType {
    ServerHandshake = 1,
    ClientHandshake = 2,
    Control = 3,
    Data = 4,
}

#[derive(Debug, Clone, DekuRead, DekuWrite, PartialEq, Eq)]
#[deku(type = "u32")]
pub enum VideoCodec {
    H264 = 0,
    H265 = 1,
    Yuv = 2,
    Rgb = 3,
}

#[derive(Debug, Clone, DekuRead, DekuWrite, Eq, PartialEq, Default)]
pub struct VideoControlFlags {
    /// Packet contains last displayed frame rendered
    /// Bit 31 / Mask LE 0x80000000 BE 0x80
    #[deku(bits = "1")]
    pub last_displayed_frame_rendered: bool,
    /// Requesting keyframes
    /// Bit 29 / Mask LE 0x20000000 BE 0x20
    #[deku(pad_bits_before = "1", bits = "1")]
    pub request_keyframes: bool,
    /// Start stream
    /// Bit 28 / Mask LE 0x10000000 BE 0x10
    #[deku(bits = "1")]
    pub start_stream: bool,
    /// Stop stream
    /// Bit 27 / Mask LE 0x08000000 BE 0x08
    #[deku(bits = "1")]
    pub stop_stream: bool,
    /// Packet contains queue depth
    /// Bit 26 / Mask LE 0x04000000 BE 0x04
    #[deku(bits = "1")]
    pub queue_depth: bool,
    /// Packet contains lost frames
    /// Bit 25 / Mask LE 0x02000000 BE 0x02
    #[deku(bits = "1")]
    pub lost_frames: bool,
    /// Packet contains last displayed frame
    /// Bit 24 / Mask LE 0x01000000 BE 0x01
    #[deku(bits = "1")]
    pub last_displayed_frame: bool,
    /// Packet contains smooth rendering settings
    /// Bit 20 / Mask LE 0x00100000 BE 0x1000
    #[deku(pad_bits_before = "3", bits = "1")]
    pub smooth_rendering_settings_sent: bool,
    /// Packet contains bitrate update
    /// Bit 18 / Mask LE 0x00040000 BE 0x400
    #[deku(pad_bits_before = "1", bits = "1")]
    pub bitrate_update: bool,
    /// Packet contains video format change
    /// Bit 17 / Mask LE 0x00020000 BE 0x200
    #[deku(pad_bits_after = "16", bits = "1")]
    pub video_format_change: bool,
}

#[derive(Debug, Clone, DekuRead, DekuWrite, Eq, PartialEq, Default)]
pub struct VideoDataFlags {
    /// Jitter info
    /// Bit 28 / Mask LE 0x10000000 BE 0x10
    #[deku(pad_bits_before = "3", bits = "1")]
    pub jitter_info: bool,
    /// Hashed
    /// Bit 27 / Mask LE 0x08000000 BE 0x08
    #[deku(pad_bits_after = "26", bits = "1")]
    pub hashed: bool,
}

#[derive(Debug, Clone, DekuRead, DekuWrite, PartialEq, Eq)]
pub struct RGBVideoFormat {
    pub bpp: u32,
    pub unknown: u32,
    pub red_mask: u64,
    pub green_mask: u64,
    pub blue_mask: u64,
}

#[derive(Debug, Clone, DekuRead, DekuWrite, PartialEq, Eq)]
pub struct VideoFormat {
    pub fps: u32,
    pub width: u32,
    pub height: u32,
    pub codec: VideoCodec,
    #[deku(cond = "*codec == VideoCodec::Rgb")]
    pub rgb_format: Option<RGBVideoFormat>,
}

#[derive(Debug, Clone, DekuRead, DekuWrite, PartialEq, Eq)]
pub struct VideoServerHandshake {
    pub unknown1: u32,
    pub unknown2: u32,
    pub protocol_version: u32,
    pub screen_width: u32,
    pub screen_height: u32,
    pub fps: u32,
    pub reference_timestamp: u64,
    #[deku(update = "self.formats.len()")]
    pub format_count: u32,
    #[deku(count = "format_count")]
    pub formats: Vec<VideoFormat>,
}

#[derive(Debug, Clone, DekuRead, DekuWrite, PartialEq, Eq)]
pub struct VideoClientHandshake {
    pub unknown1: u32,
    pub unknown2: u32,
    pub initial_frame_id: u32,
    pub requested_format: VideoFormat,
}

#[derive(Debug, Clone, DekuRead, DekuWrite, PartialEq, Eq)]
pub struct VideoControl {
    pub flags: VideoControlFlags,
    // Tuple
    #[deku(cond = "flags.last_displayed_frame && flags.last_displayed_frame_rendered")]
    pub last_displayed_frame: Option<(u32, i64)>,
    // Tuple of (first, last) lost frame
    #[deku(cond = "flags.queue_depth")]
    pub queue_depth: Option<u32>,
    #[deku(cond = "flags.lost_frames")]
    pub lost_frames: Option<(u32, u32)>,
    #[deku(cond = "flags.bitrate_update")]
    pub bitrate_update: Option<u32>,
    #[deku(cond = "flags.video_format_change")]
    pub video_format_update: Option<VideoFormat>,
    #[deku(cond = "flags.smooth_rendering_settings_sent")]
    pub smooth_rendering_settings: Option<(u64, u64, u64)>,
}

#[derive(Debug, Clone, DekuRead, DekuWrite, PartialEq, Eq)]
pub struct VideoData {
    pub unknown1: u32,
    pub unknown2: u32,
    pub flags: VideoDataFlags,
    pub frame_id: u32,
    pub timestamp: u64,
    pub packet_count: u32,
    pub total_size: u32,
    pub metadata_size: u32,
    pub offset: u32,
    pub unknown3: u32,
    #[deku(update = "self.data.len()")]
    pub data_size: u32,
    #[deku(count = "data_size")]
    pub data: Vec<u8>,
}

#[derive(Debug, DekuRead, DekuWrite, Clone, PartialEq, Eq)]
pub struct VideoPacket {
    pub packet_type: VideoPacketType,
    #[deku(cond = "*packet_type == VideoPacketType::ServerHandshake")]
    pub server_handshake: Option<VideoServerHandshake>,
    #[deku(cond = "*packet_type == VideoPacketType::ClientHandshake")]
    pub client_handshake: Option<VideoClientHandshake>,
    #[deku(cond = "*packet_type == VideoPacketType::Control")]
    pub control: Option<VideoControl>,
    #[deku(cond = "*packet_type == VideoPacketType::Data")]
    pub data: Option<VideoData>,
}

#[cfg(test)]
mod test {
    use std::convert::TryInto;

    use super::*;

    #[test]
    fn deserialize_video_server_handshake() {
        let data = include_bytes!("../../testdata/video_server_handshake.bin");
        let (rest, packet) = VideoPacket::from_bytes((&data[20..], 0))
            .expect("Failed to parse VideoServerHandshake packet");

        assert_eq!(rest.1, 0);

        let server_hs = packet
            .server_handshake
            .expect("Server handshake not parsed");

        assert_eq!(server_hs.protocol_version, 6);
        assert_eq!(server_hs.screen_width, 1280);
        assert_eq!(server_hs.screen_height, 720);
        assert_eq!(server_hs.fps, 60);
        assert_eq!(server_hs.reference_timestamp, 1613399625116);
        assert_eq!(server_hs.format_count, 1);
        assert_eq!(server_hs.formats.len(), 1);
        assert_eq!(server_hs.formats[0].fps, 60);
        assert_eq!(server_hs.formats[0].width, 1280);
        assert_eq!(server_hs.formats[0].height, 720);
        assert_eq!(server_hs.formats[0].codec, VideoCodec::H264);
        assert_eq!(server_hs.formats[0].rgb_format, None);
    }

    #[test]
    fn deserialize_video_client_handshake() {
        let data = include_bytes!("../../testdata/video_client_handshake.bin");
        let (rest, packet) = VideoPacket::from_bytes((&data[12..], 0))
            .expect("Failed to parse VideoClientHandshake packet");

        let client_hs = packet
            .client_handshake
            .expect("Client handshake not parsed");

        assert_eq!(rest.1, 0);

        assert_eq!(client_hs.initial_frame_id, 1808917930);
        assert_eq!(client_hs.requested_format.fps, 60);
        assert_eq!(client_hs.requested_format.width, 1280);
        assert_eq!(client_hs.requested_format.height, 720);
        assert_eq!(client_hs.requested_format.codec, VideoCodec::H264);
        assert_eq!(client_hs.requested_format.rgb_format, None);
    }

    #[test]
    #[ignore]
    fn deserialize_video_control() {
        let data = include_bytes!("../../testdata/video_control.bin");
        let (rest, packet) =
            VideoPacket::from_bytes((&data[12..], 0)).expect("Failed to parse VideoControl packet");

        let video_control = packet.control.expect("Control not parsed");

        assert_eq!(rest.1, 0);

        assert!(video_control.flags.start_stream);
    }

    #[test]
    #[ignore]
    fn deserialize_video_data() {
        let data = include_bytes!("../../testdata/video_data.bin");
        let (rest, packet) =
            VideoPacket::from_bytes((&data[12..], 0)).expect("Failed to parse VideoData packet");

        assert_eq!(rest.1, 0);

        let video_data = packet.data.expect("Data not parsed");

        // assert_eq!(video_data.flags.);
        assert_eq!(video_data.frame_id, 1808917930);
        assert_eq!(video_data.timestamp, 3177068);
        assert_eq!(video_data.packet_count, 9);
        assert_eq!(video_data.total_size, 11277);
        assert_eq!(video_data.metadata_size, 9);
        assert_eq!(video_data.offset, 0);
        assert_eq!(video_data.unknown3, 9);
        assert_eq!(video_data.data_size, 1245);
        assert_eq!(video_data.data.len(), 1245);
    }

    #[test]
    fn parse_video_control_flags() {
        fn create_flag(val: u32) -> VideoControlFlags {
            let val_bytes: [u8; 4] = val.to_le_bytes();
            let (_, flags) =
                VideoControlFlags::from_bytes((&val_bytes, 0)).expect("Failed to create flags");

            flags
        }
        assert!(create_flag(0x01).last_displayed_frame);
        assert!(create_flag(0x02).lost_frames);
        assert!(create_flag(0x04).queue_depth);
        assert!(create_flag(0x08).stop_stream);
        assert!(create_flag(0x10).start_stream);
        assert!(create_flag(0x20).request_keyframes);
        assert!(create_flag(0x80).last_displayed_frame_rendered);
        assert!(create_flag(0x200).video_format_change);
        assert!(create_flag(0x400).bitrate_update);
        assert!(create_flag(0x1000).smooth_rendering_settings_sent);
    }

    #[test]
    fn serialize_video_control_flags() {
        fn get_value(flags: VideoControlFlags) -> u32 {
            let val: [u8; 4] = flags
                .to_bytes()
                .expect("Failed to conver to bytes")
                .as_slice()
                .try_into()
                .expect("slice with incorrect length");
            u32::from_le_bytes(val)
        }

        let last_displayed_frame = VideoControlFlags {
            last_displayed_frame: true,
            ..Default::default()
        };
        let lost_frames = VideoControlFlags {
            lost_frames: true,
            ..Default::default()
        };
        let queue_depth = VideoControlFlags {
            queue_depth: true,
            ..Default::default()
        };
        let stop_stream = VideoControlFlags {
            stop_stream: true,
            ..Default::default()
        };
        let start_stream = VideoControlFlags {
            start_stream: true,
            ..Default::default()
        };
        let request_keyframes = VideoControlFlags {
            request_keyframes: true,
            ..Default::default()
        };
        let last_displayed_frame_rendered = VideoControlFlags {
            last_displayed_frame_rendered: true,
            ..Default::default()
        };
        let video_format_change = VideoControlFlags {
            video_format_change: true,
            ..Default::default()
        };
        let bitrate_update = VideoControlFlags {
            bitrate_update: true,
            ..Default::default()
        };
        let smooth_rendering_settings_sent = VideoControlFlags {
            smooth_rendering_settings_sent: true,
            ..Default::default()
        };

        assert_eq!(get_value(last_displayed_frame), 0x01);
        assert_eq!(get_value(lost_frames), 0x02);
        assert_eq!(get_value(queue_depth), 0x04);
        assert_eq!(get_value(stop_stream), 0x08);
        assert_eq!(get_value(start_stream), 0x10);
        assert_eq!(get_value(request_keyframes), 0x20);
        assert_eq!(get_value(last_displayed_frame_rendered), 0x80);
        assert_eq!(get_value(video_format_change), 0x200);
        assert_eq!(get_value(bitrate_update), 0x400);
        assert_eq!(get_value(smooth_rendering_settings_sent), 0x1000);
    }
}
