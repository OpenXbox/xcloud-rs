use std::{convert::{TryInto, From}, io::{Read, Seek, Write}};
use byteorder::*;
use bitflags::bitflags;

use crate::packets::serializing::{Deserialize, Serialize};

use super::message;

type Error = Box<dyn std::error::Error>;
type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, PartialEq)]
#[repr(u32)]
pub enum VideoPacketType {
    ServerHandshake = 1,
    ClientHandshake = 2,
    Control = 3,
    Data = 4,
}

impl From<u32> for VideoPacketType {
    fn from(value: u32) -> Self {
        let z: VideoPacketType = unsafe { ::std::mem::transmute(value) };

        z
    }
}

#[derive(Debug, Clone, PartialEq)]
#[repr(u32)]
pub enum VideoCodec {
    H264 = 0,
    H265 = 1,
    YUV = 2,
    RGB = 3,
}

impl From<u32> for VideoCodec {
    fn from(value: u32) -> Self {
        let z: VideoCodec = unsafe { ::std::mem::transmute(value) };

        z
    }
}

bitflags! {
    pub struct VideoControlFlags : u32 {
        const LAST_DISPLAYED_FRAME = 0x01;
        const LOST_FRAMES = 0x02;
        const QUEUE_DEPTH = 0x04;
        const STOP_STREAM = 0x08;
        const START_STREAM = 0x10;
        const REQUEST_KEYFRAMES = 0x20;
        const LAST_DISPLAYED_FRAME_RENDERED = 0x80;
        const SMOOTH_RENDERING_SETTINGS_SENT = 0x1000;
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RGBVideoFormat {
    pub bpp: u32,
    pub unknown: u32,
    pub red_mask: u64,
    pub green_mask: u64,
    pub blue_mask: u64,
}

impl Deserialize for RGBVideoFormat {
    fn deserialize<T: Read + Seek>(reader: &mut T) -> Result<Self> {
        let bpp = reader.read_u32::<LittleEndian>()?;
        let unknown = reader.read_u32::<LittleEndian>()?;
        let red_mask = reader.read_u64::<LittleEndian>()?;
        let green_mask = reader.read_u64::<LittleEndian>()?;
        let blue_mask = reader.read_u64::<LittleEndian>()?;

        Ok(Self {
            bpp,
            unknown,
            red_mask,
            green_mask,
            blue_mask
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct VideoFormat {
    pub fps: u32,
    pub width: u32,
    pub height: u32,
    pub codec: u32,
    pub rgb_format: Option<RGBVideoFormat>,
}

impl Deserialize for VideoFormat {
    fn deserialize<T: Read + Seek>(reader: &mut T) -> Result<Self> {
        let fps = reader.read_u32::<LittleEndian>()?;
        let width = reader.read_u32::<LittleEndian>()?;
        let height = reader.read_u32::<LittleEndian>()?;
        let codec = reader.read_u32::<LittleEndian>()?;

        let rgb_format = match codec.try_into()? {
            VideoCodec::RGB => {
                Some(RGBVideoFormat::deserialize(reader)?)
            }
            _ => None,
        };

        Ok(Self {
            fps,
            width,
            height,
            codec,
            rgb_format
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct VideoServerHandshake {
    pub unknown1: u32,
    pub unknown2: u32,
    pub protocol_version: u32,
    pub screen_width: u32,
    pub screen_height: u32,
    pub fps: u32,
    pub reference_timestamp: u64,
    pub format_count: u32,
    pub formats: Vec<VideoFormat>
}

impl Deserialize for VideoServerHandshake {
    fn deserialize<T: Read + Seek>(reader: &mut T) -> Result<Self> {
        let unknown1 = reader.read_u32::<LittleEndian>()?;
        let unknown2 = reader.read_u32::<LittleEndian>()?;
        let protocol_version = reader.read_u32::<LittleEndian>()?;
        let screen_width = reader.read_u32::<LittleEndian>()?;
        let screen_height = reader.read_u32::<LittleEndian>()?;
        let fps = reader.read_u32::<LittleEndian>()?;
        let reference_timestamp = reader.read_u64::<LittleEndian>()?;
        let format_count = reader.read_u32::<LittleEndian>()?;

        let mut formats: Vec<VideoFormat> = Vec::<VideoFormat>::new();
        for _ in 0..format_count {
            formats.push(VideoFormat::deserialize(reader)?);
        }

        Ok(Self {
            unknown1,
            unknown2,
            protocol_version,
            screen_width,
            screen_height,
            fps,
            reference_timestamp,
            format_count,
            formats: formats
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct VideoClientHandshake {
    pub unknown1: u32,
    pub unknown2: u32,
    pub initial_frame_id: u32,
    pub requested_format: VideoFormat,
}

impl Deserialize for VideoClientHandshake {
    fn deserialize<T: Read + Seek>(reader: &mut T) -> Result<Self> {
        let unknown1 = reader.read_u32::<LittleEndian>()?;
        let unknown2 = reader.read_u32::<LittleEndian>()?;
        let initial_frame_id = reader.read_u32::<LittleEndian>()?;
        let requested_format = VideoFormat::deserialize(reader)?;

        Ok(Self {
            unknown1,
            unknown2,
            initial_frame_id,
            requested_format
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct VideoControl {
    pub flags: u32,
    pub last_displayed_frame: Option<u32>,
    pub last_displayed_frame_rendered: Option<u32>,
    // Tuple of (first, last) lost frame
    pub lost_frames: Option<(u32, u32)>,
    pub queue_depth: Option<u32>,
}

impl Deserialize for VideoControl {
    fn deserialize<T: Read + Seek>(reader: &mut T) -> Result<Self> {
        let flags = reader.read_u32::<LittleEndian>()?;

        let flags_ = VideoControlFlags::from_bits(flags).unwrap();

        let last_displayed_frame = {
            if flags_.contains(VideoControlFlags::LAST_DISPLAYED_FRAME) {
                Some(reader.read_u32::<LittleEndian>()?)
            }
            else {
                None
            }
        };

        let last_displayed_frame_rendered = {
            if flags_.contains(VideoControlFlags::LAST_DISPLAYED_FRAME_RENDERED) {
                Some(reader.read_u32::<LittleEndian>()?)
            }
            else {
                None
            }
        };

        let lost_frames = {
            if flags_.contains(VideoControlFlags::LOST_FRAMES) {
                Some((reader.read_u32::<LittleEndian>()?, reader.read_u32::<LittleEndian>()?))
            }
            else {
                None
            }
        };

        let queue_depth = {
            if flags_.contains(VideoControlFlags::QUEUE_DEPTH) {
                Some(reader.read_u32::<LittleEndian>()?)
            }
            else {
                None
            }
        };

        Ok(Self {
            flags,
            last_displayed_frame,
            last_displayed_frame_rendered,
            lost_frames,
            queue_depth
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct VideoData {
    pub unknown1: u32,
    pub unknown2: u32,
    pub flags: u32,
    pub frame_id: u32,
    pub timestamp: u64,
    pub packet_count: u32,
    pub total_size: u32,
    pub metadata_size: u32,
    pub offset: u32,
    pub unknown3: u32,
    pub data_size: u32,
    pub data: Vec<u8>
}

impl Deserialize for VideoData {
    fn deserialize<T: Read + Seek>(reader: &mut T) -> Result<Self> {
        let unknown1 = reader.read_u32::<LittleEndian>()?;
        let unknown2 = reader.read_u32::<LittleEndian>()?;
        let flags = reader.read_u32::<LittleEndian>()?;
        let frame_id = reader.read_u32::<LittleEndian>()?;
        let timestamp = reader.read_u64::<LittleEndian>()?;
        let packet_count = reader.read_u32::<LittleEndian>()?;
        let total_size = reader.read_u32::<LittleEndian>()?;
        let metadata_size = reader.read_u32::<LittleEndian>()?;
        let offset = reader.read_u32::<LittleEndian>()?;
        let unknown3 = reader.read_u32::<LittleEndian>()?;
        let data_size = reader.read_u32::<LittleEndian>()?;

        let data = {
            let mut data = vec![0; data_size as usize];
            reader.read_exact(&mut data)?;

            data
        };

        Ok(Self {
            unknown1,
            unknown2,
            flags,
            frame_id,
            timestamp,
            packet_count,
            total_size,
            metadata_size,
            offset,
            unknown3,
            data_size,
            data
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum VideoPacket {
    ServerHandshake(VideoServerHandshake),
    ClientHandshake(VideoClientHandshake),
    Control(VideoControl),
    Data(VideoData)
}

impl Deserialize for VideoPacket {
    fn deserialize<T: Read + Seek>(reader: &mut T) -> Result<Self> {
        let packet_type = reader.read_u32::<LittleEndian>()?.try_into()?;

        let packet  = match packet_type {
            VideoPacketType::ServerHandshake => {
                VideoPacket::ServerHandshake(VideoServerHandshake::deserialize(reader)?)
            },
            VideoPacketType::ClientHandshake => {
                VideoPacket::ClientHandshake(VideoClientHandshake::deserialize(reader)?)
            },
            VideoPacketType::Control => {
                VideoPacket::Control(VideoControl::deserialize(reader)?)
            },
            VideoPacketType::Data => {
                VideoPacket::Data(VideoData::deserialize(reader)?)
            },
        };

        Ok(packet)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn deserialize_video_server_handshake() {
        let data = include_bytes!("../../testdata/video_server_handshake.bin");
        let mut reader = Cursor::new(&data[20..]);

        let packet = VideoPacket::deserialize(&mut reader).
            expect("Failed to deserialize packet");

        println!("{:?}", packet);
        match packet {
            VideoPacket::ServerHandshake(server_hs_pkt) => {
                assert_eq!(server_hs_pkt.protocol_version, 6);
                assert_eq!(server_hs_pkt.screen_width, 1280);
                assert_eq!(server_hs_pkt.screen_height, 720);
                assert_eq!(server_hs_pkt.fps, 60);
                assert_eq!(server_hs_pkt.reference_timestamp, 1613399625116);
                assert_eq!(server_hs_pkt.format_count, 1);
                assert_eq!(server_hs_pkt.formats.len(), 1);
                assert_eq!(server_hs_pkt.formats[0].fps, 60);
                assert_eq!(server_hs_pkt.formats[0].width, 1280);
                assert_eq!(server_hs_pkt.formats[0].height, 720);
                assert_eq!(server_hs_pkt.formats[0].codec, 0);
                assert_eq!(server_hs_pkt.formats[0].rgb_format, None);
            },
            _ => panic!("Parsed into invalid packet")
        }
    }

    #[test]
    fn deserialize_video_client_handshake() {
        let data = include_bytes!("../../testdata/video_client_handshake.bin");
        let mut reader = Cursor::new(&data[12..]);

        let packet = VideoPacket::deserialize(&mut reader).
            expect("Failed to deserialize packet");

        println!("{:?}", packet);
        match packet {
            VideoPacket::ClientHandshake(client_hs_pkt) => {
                assert_eq!(client_hs_pkt.initial_frame_id, 1808917930);
                assert_eq!(client_hs_pkt.requested_format.fps, 60);
                assert_eq!(client_hs_pkt.requested_format.width, 1280);
                assert_eq!(client_hs_pkt.requested_format.height, 720);
                assert_eq!(client_hs_pkt.requested_format.codec, 0);
                assert_eq!(client_hs_pkt.requested_format.rgb_format, None);
            },
            _ => panic!("Parsed into invalid packet")
        }
    }

    #[test]
    fn deserialize_video_control() {
        let data = include_bytes!("../../testdata/video_control.bin");
        let mut reader = Cursor::new(&data[12..]);

        let packet = VideoPacket::deserialize(&mut reader).
            expect("Failed to deserialize packet");

        println!("{:?}", packet);
        match packet {
            VideoPacket::Control(control_pkt) => {
                let flags: VideoControlFlags = VideoControlFlags::from_bits(control_pkt.flags)
                    .expect("Failed to parse VideoControlFlags");

                panic!("VideoControl struct is not correct yet");
                // assert!(flags.contains(VideoControlFlags::START_STREAM));
            },
            _ => panic!("Parsed into invalid packet")
        }
    }

    #[test]
    fn deserialize_video_data() {
        let data = include_bytes!("../../testdata/video_data.bin");
        let mut reader = Cursor::new(&data[12..]);

        let packet = VideoPacket::deserialize(&mut reader).
            expect("Failed to deserialize packet");

        println!("{:?}", packet);
        match packet {
            VideoPacket::Data(data_pkt) => {
                assert_eq!(data_pkt.flags, 4);
                assert_eq!(data_pkt.frame_id, 1808917930);
                assert_eq!(data_pkt.timestamp, 3177068);
                assert_eq!(data_pkt.packet_count, 9);
                assert_eq!(data_pkt.total_size, 11277);
                assert_eq!(data_pkt.metadata_size, 9);
                assert_eq!(data_pkt.offset, 0);
                assert_eq!(data_pkt.unknown3, 9);
                assert_eq!(data_pkt.data_size, 1245);
                assert_eq!(data_pkt.data.len(), 1245);
            },
            _ => panic!("Parsed into invalid packet")
        }
    }
}