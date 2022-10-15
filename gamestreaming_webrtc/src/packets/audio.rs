use deku::prelude::*;

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "little")]
struct AudioFrame {
    id: u32,
    timestamp: u32,
    size: u32,
    #[deku(count = "size")]
    buffer: Vec<u8>,
}

#[cfg(test)]
mod tests {}
