#[derive(Debug, Clone, PartialEq)]
pub enum VideoPacketType {
    ServerHandshake = 1,
    ClientHandshake = 2,
    Control = 3,
    Data = 4,
}

#[derive(Debug, Clone, PartialEq)]
pub enum VideoCodec {
    H264 = 0,
    H265 = 1,
    YUV = 2,
    RGB = 3,
}

#[derive(Debug, Clone, PartialEq)]
pub enum VideoControlFlags {
    LastDisplayedFrame = 0x01,
    LostFrames = 0x02,
    QueueDepth = 0x04,
    StopStream = 0x08,
    StartStream = 0x10,
    RequestKeyframes = 0x20,
    LastDisplayedFrameRendered = 0x80,
    SmoothRenderingSettingsSent = 0x1000,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RGBVideoFormat {
    pub bpp: u32,
    pub unknown: u32,
    pub red_mask: u64,
    pub green_mask: u64,
    pub blue_mask: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VideoFormat {
    pub fps: u32,
    pub width: u32,
    pub height: u32,
    pub codec: u32,
    pub rgb_format: Option<RGBVideoFormat>,
}

#[derive(Debug, PartialEq)]
pub struct VideoServerHandshake {
    pub protocol_version: u32,
    pub screen_width: u32,
    pub screen_height: u32,
    pub reference_timestamp: u64,
    pub format_count: u32,
    pub formats: [VideoFormat]
}

#[derive(Debug, Clone, PartialEq)]
pub struct VideoClientHandshake {
    pub initial_frame_id: u32,
    pub requested_format: VideoFormat,
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

#[derive(Debug, Clone, PartialEq)]
pub struct VideoData {
    pub flags: u32,
    pub frame_id: u32,
    pub timestamp: u64,
    pub metadata_size: u32,
    pub metadata: Vec<u8>,
    pub data_size: u32,
    pub offset: u32,
}