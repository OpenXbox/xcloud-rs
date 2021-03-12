#[derive(Debug, Clone, PartialEq)]
pub struct VideoData {
    flags: u32,
    frame_id: u32,
    timestamp: u32,
    metadata_size: u16,
    data_size: u16,
    offset: u32,
}
