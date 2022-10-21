use deku::prelude::*;

#[allow(non_snake_case)]
#[derive(Copy, Clone, Default, DekuRead, DekuWrite, Debug, Eq, PartialEq)]
#[deku(endian = "little")]
pub struct InputReportType {
    /// Bitmask: 0x80
    #[deku(bits = "1")]
    Vibration: bool,
    /// Bitmask: 0x40
    #[deku(bits = "1")]
    Keyboard: bool,
    /// Bitmask: 0x20
    #[deku(bits = "1")]
    Mouse: bool,
    /// Bitmask: 0x10
    #[deku(bits = "1")]
    ServerMetadata: bool,
    /// Bitmask: 0x08
    #[deku(bits = "1")]
    ClientMetadata: bool,
    /// Bitmask: 0x04
    #[deku(bits = "1")]
    Unused: bool,
    /// Bitmask: 0x02
    #[deku(bits = "1")]
    GamepadReport: bool,
    /// Bitmask: 0x01
    #[deku(bits = "1")]
    Metadata: bool,
}

#[allow(non_snake_case)]
#[derive(Copy, Clone, DekuRead, DekuWrite, Debug, Eq, PartialEq)]
#[deku(endian = "little")]
pub struct GamepadButton {
    /// Bitmask: 0x8000
    #[deku(bits = "1")]
    Unused: bool,
    /// Bitmask: 0x4000
    #[deku(bits = "1")]
    RightThumb: bool,
    /// Bitmask: 0x2000
    #[deku(bits = "1")]
    LeftThumb: bool,
    /// Bitmask: 0x1000
    #[deku(bits = "1")]
    RightShoulder: bool,
    /// Bitmask: 0x800
    #[deku(bits = "1")]
    LeftShoulder: bool,
    /// Bitmask: 0x400
    #[deku(bits = "1")]
    DPadRight: bool,
    /// Bitmask: 0x200
    #[deku(bits = "1")]
    DPadLeft: bool,
    /// Bitmask: 0x100
    #[deku(bits = "1")]
    DPadDown: bool,
    /// Bitmask: 0x80
    #[deku(bits = "1")]
    DPadUp: bool,
    /// Bitmask: 0x40
    #[deku(bits = "1")]
    Y: bool,
    /// Bitmask: 0x20
    #[deku(bits = "1")]
    X: bool,
    /// Bitmask: 0x10
    #[deku(bits = "1")]
    B: bool,
    /// Bitmask: 0x08
    #[deku(bits = "1")]
    A: bool,
    /// Bitmask: 0x04
    #[deku(bits = "1")]
    View: bool,
    /// Bitmask: 0x02
    #[deku(bits = "1")]
    Menu: bool,
    /// Bitmask: 0x01
    #[deku(bits = "1")]
    Nexus: bool,
}

#[derive(Debug, Eq, PartialEq, DekuRead, DekuWrite)]
pub struct VibrationReport {
    /// Rumble Type: 0 = FourMotorRumble
    pub rumble_type: u8,
    pub gamepad_id: u8,

    pub left_motor_percent: u8,
    pub right_motor_percent: u8,
    pub left_trigger_motor_percent: u8,
    pub right_trigger_motor_percent: u8,
    pub duration_ms: u16,
    pub delay_ms: u16,
    pub repeat: u8,
}

#[derive(Debug, Eq, PartialEq, DekuRead, DekuWrite)]
pub struct InputMetadataEntry {
    pub server_data_key: u32,
    pub first_frame_packet_arrival_time_ms: u32,
    pub frame_submitted_time_ms: u32,
    pub frame_decoded_time_ms: u32,
    pub frame_rendered_time_ms: u32,
    pub frame_packet_time: u32,
    pub frame_date_now: u32,
}

#[derive(Debug, Eq, PartialEq, DekuRead, DekuWrite)]
pub struct MetadataReport {
    #[deku(update = "self.metadata.len()")]
    pub queue_len: u8,
    #[deku(count = "queue_len")]
    pub metadata: Vec<InputMetadataEntry>,
}

#[derive(Debug, Eq, PartialEq, DekuRead, DekuWrite)]
pub struct GamepadData {
    pub gamepad_index: u8,
    pub button_mask: GamepadButton,
    pub left_thumb_x: i16,
    pub left_thumb_y: i16,
    pub right_thumb_x: i16,
    pub right_thumb_y: i16,
    pub left_trigger: u16,
    pub right_trigger: u16,
    pub physical_physicality: u32,
    pub virtual_physicality: u32,
}

#[derive(Debug, Eq, PartialEq, DekuRead, DekuWrite)]
pub struct GamepadReport {
    #[deku(update = "self.gamepad_data.len()")]
    pub queue_len: u8,
    #[deku(count = "queue_len")]
    pub gamepad_data: Vec<GamepadData>,
}

#[derive(Debug, Default, Eq, PartialEq, DekuRead, DekuWrite)]
pub struct ClientMetadataReport {
    pub metadata: u8,
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
pub struct SequenceInfo {
    sequence_num: u32,
    timestamp: f64,
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
pub struct InputPacket {
    report_type: InputReportType,
    #[deku(cond = "!report_type.Vibration")]
    // Skip sequence info on vibration packets
    seq_info: Option<SequenceInfo>,
    #[deku(cond = "report_type.Metadata")]
    metadata_report: Option<MetadataReport>,
    #[deku(cond = "report_type.GamepadReport")]
    gamepad_report: Option<GamepadReport>,
    #[deku(cond = "report_type.ClientMetadata")]
    client_metadata_report: Option<ClientMetadataReport>,
    #[deku(cond = "report_type.Vibration")]
    vibration_report: Option<VibrationReport>,
}

impl InputPacket {
    pub fn new(
        sequence_num: u32,
        timestamp: f64,
        metadata_report: Option<MetadataReport>,
        gamepad_report: Option<GamepadReport>,
        client_metadata_report: Option<ClientMetadataReport>,
    ) -> Self {
        let report_type = {
            // Create initial report type with no bits set
            let mut tmp_type: InputReportType = InputReportType::default();

            // Check which data will be contained
            if metadata_report.is_some() {
                tmp_type.Metadata = true;
            }
            if gamepad_report.is_some() {
                tmp_type.GamepadReport = true;
            }
            if client_metadata_report.is_some() {
                tmp_type.ClientMetadata = true;
            }
            tmp_type
        };

        Self {
            report_type,
            seq_info: Some(SequenceInfo {
                sequence_num,
                timestamp,
            }),
            metadata_report,
            gamepad_report,
            client_metadata_report,
            vibration_report: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use deku::bitvec::BitSlice;

    use super::*;

    #[test]
    fn deserialize_vibration_report() {
        let test_data = vec![
            0x00, 0x00, 0xF1, 0xF2, 0xF3, 0xF4, 0x50, 0x01, 0xFF, 0x01, 0x10,
        ];
        let (rest, parsed) = VibrationReport::from_bytes((&test_data, 0))
            .expect("Failed to deserialize input rumble packet");

        println!("{:?}", rest);
        assert!(rest.0.is_empty());
        assert_eq!(rest.1, 0);
        assert_eq!(parsed.rumble_type, 0x00);
        assert_eq!(parsed.gamepad_id, 0x00);
        assert_eq!(parsed.left_motor_percent, 0xF1);
        assert_eq!(parsed.right_motor_percent, 0xF2);
        assert_eq!(parsed.left_trigger_motor_percent, 0xF3);
        assert_eq!(parsed.right_trigger_motor_percent, 0xF4);
        assert_eq!(parsed.duration_ms, 0x150);
        assert_eq!(parsed.delay_ms, 0x1FF);
        assert_eq!(parsed.repeat, 0x10);
    }

    #[test]
    fn deserialize_input_packet() {
        let test_data = vec![
            0x80, 0x00, 0x00, 0xF1, 0xF2, 0xF3, 0xF4, 0x50, 0x01, 0xFF, 0x01, 0x10,
        ];

        let (rest, parsed) = InputPacket::from_bytes((&test_data, 0))
            .expect("Failed to deserialize input rumble packet");

        println!("{:?}", rest);
        assert!(rest.0.is_empty());
        assert_eq!(rest.1, 0);

        assert!(parsed.seq_info.is_none());
        assert!(parsed.metadata_report.is_none());
        assert!(parsed.gamepad_report.is_none());
        assert!(parsed.client_metadata_report.is_none());
        assert!(parsed.report_type.Vibration);

        let vibration_payload = parsed.vibration_report.expect("No vibration payload");
        assert_eq!(vibration_payload.rumble_type, 0x00);
        assert_eq!(vibration_payload.gamepad_id, 0x00);
        assert_eq!(vibration_payload.left_motor_percent, 0xF1);
        assert_eq!(vibration_payload.right_motor_percent, 0xF2);
        assert_eq!(vibration_payload.left_trigger_motor_percent, 0xF3);
        assert_eq!(vibration_payload.right_trigger_motor_percent, 0xF4);
        assert_eq!(vibration_payload.duration_ms, 0x150);
        assert_eq!(vibration_payload.delay_ms, 0x1FF);
        assert_eq!(vibration_payload.repeat, 0x10);
    }

    #[test]
    fn parse_input_report_type() {
        let data = [0x41u8];
        let bitslice = BitSlice::from_slice(&data).expect("Failed to create bitslice");
        let (rest, parsed) =
            InputReportType::read(bitslice, ()).expect("Failed to parse input report type");

        assert!(rest.is_empty());

        assert!(parsed.Keyboard);
        assert!(parsed.Metadata);
        assert!(!parsed.GamepadReport);
        assert!(!parsed.ClientMetadata);
        assert!(!parsed.Vibration);
        assert!(!parsed.Mouse);
    }

    #[test]
    fn parse_gamepad_button() {
        // A, DPadRight, LeftThumb
        let data = [0x24, 0x08u8];

        let bitslice = BitSlice::from_slice(&data).expect("Failed to create bitslice");
        let (rest, parsed) =
            GamepadButton::read(bitslice, ()).expect("Failed to parse gamepad button flags");

        assert!(rest.is_empty());

        println!("{:?}", parsed);

        assert!(parsed.A);
        assert!(parsed.DPadRight);
        assert!(parsed.LeftThumb);
        assert!(!parsed.RightThumb);
        assert!(!parsed.B);
        assert!(!parsed.X);
        assert!(!parsed.Y);
        assert!(!parsed.DPadLeft);
    }
}
