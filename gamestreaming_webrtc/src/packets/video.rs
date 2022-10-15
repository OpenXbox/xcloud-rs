use deku::prelude::*;

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "little")]
struct VideoFrame {
    id: u32,
    timestamp: u32,
    size: u32,
    offset: u32,
    server_data_key: u32,
    is_keyframe: u8,
    #[deku(count = "size")]
    buffer: Vec<u8>,
}

#[cfg(test)]
mod tests {}
