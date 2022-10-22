use deku::prelude::*;

#[derive(Debug, Clone, DekuRead, DekuWrite, PartialEq, Eq)]
#[deku(type = "u32")]
pub enum QosPacketType {
    ServerHandshake = 1,
    ClientHandshake = 2,
    Control = 3,
    Data = 4,
    ServerPolicy = 5,
    ClientPolicy = 6,
}

#[derive(Debug, Clone, DekuRead, DekuWrite, Eq, PartialEq, Default)]
pub struct QosControlFlags {
    // Bit 1 / Mask 0x01
    #[deku(pad_bits_before = "7", bits = "1")]
    pub reinitialize: bool,
}

#[derive(Debug, Clone, DekuRead, DekuWrite, PartialEq, Eq)]
pub struct QosServerPolicy {
    pub schema_version: u32,
    pub policy_length: u32,
    pub fragment_count: u32,
    pub offset: u32,
    pub fragment_size: u32,
}

#[derive(Debug, Clone, DekuRead, DekuWrite, PartialEq, Eq)]
pub struct QosServerHandshake {
    pub protocol_version: u32,
    #[deku(cond = "*protocol_version >= 1")]
    pub min_supported_client_version: Option<u32>,
}

#[derive(Debug, Clone, DekuRead, DekuWrite, PartialEq, Eq)]
pub struct QosClientPolicy {
    pub schema_version: u32,
}

#[derive(Debug, Clone, DekuRead, DekuWrite, PartialEq, Eq)]
pub struct QosClientHandshake {
    pub protocol_version: u32,
    pub initial_frame_id: u32,
}

#[derive(Debug, Clone, DekuRead, DekuWrite, PartialEq, Eq)]
pub struct QosControl {
    pub flags: u32,
}

#[derive(Debug, Clone, DekuRead, DekuWrite, PartialEq, Eq)]
pub struct QosData {
    pub flags: u32,
    pub frame_id: u32,
}

#[derive(Debug, Clone, DekuRead, DekuWrite, PartialEq, Eq)]
pub struct QosPacket {
    packet_type: QosPacketType,
    #[deku(cond = "*packet_type == QosPacketType::ServerHandshake")]
    server_handshake: Option<QosServerHandshake>,
    #[deku(cond = "*packet_type == QosPacketType::ClientHandshake")]
    client_handshake: Option<QosClientHandshake>,
    #[deku(cond = "*packet_type == QosPacketType::Control")]
    control: Option<QosControl>,
    #[deku(cond = "*packet_type == QosPacketType::Data")]
    data: Option<QosData>,
    #[deku(cond = "*packet_type == QosPacketType::ServerPolicy")]
    server_policy: Option<QosServerPolicy>,
    #[deku(cond = "*packet_type == QosPacketType::ClientPolicy")]
    client_policy: Option<QosClientPolicy>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_qos_control_flags() {
        fn create_flag(val: u32) -> QosControlFlags {
            let val_bytes: [u8; 4] = val.to_le_bytes();
            let (_, flags) =
                QosControlFlags::from_bytes((&val_bytes, 0)).expect("Failed to create flags");

            flags
        }
        assert!(!create_flag(0x00).reinitialize);
        assert!(create_flag(0x01).reinitialize);
    }

    #[test]
    fn serialize_qos_control_flags() {
        fn get_value(flags: QosControlFlags) -> u8 {
            let val = flags.to_bytes().expect("Failed to convert to bytes");
            assert_eq!(val.len(), 1);
            val[0]
        }

        let none = QosControlFlags::default();
        let reinitialize = QosControlFlags {
            reinitialize: true,
            ..Default::default()
        };

        assert_eq!(get_value(none), 0x00);
        assert_eq!(get_value(reinitialize), 0x01);
    }
}
