use deku::{
    bitvec::{BitSlice, BitVec, Msb0},
    prelude::*,
};
use enumflags2::{bitflags, BitFlags};

#[bitflags]
#[repr(u8)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum InputReportType {
    Metadata = 1,
    GamepadReport = 2,
    ClientMetadata = 8,
    ServerMetadata = 16,
    Mouse = 32,
    Keyboard = 64,
    Vibration = 128,
}

/// Wrapper around input report type flags
#[derive(Debug, Default, Eq, PartialEq)]
pub struct InputReportTypeFlags(BitFlags<InputReportType>);

impl<'a> DekuRead<'a> for InputReportTypeFlags {
    fn read(
        input: &'a BitSlice<Msb0, u8>,
        _ctx: (),
    ) -> Result<(&'a BitSlice<Msb0, u8>, Self), DekuError>
    where
        Self: Sized,
    {
        let (rest, flags) = u8::read(input, ())?;
        let res = BitFlags::from_bits(flags)
            .map_err(|_| DekuError::Parse("Failed to read input report type flags".into()))?;

        Ok((rest, InputReportTypeFlags(res)))
    }
}

impl DekuWrite for InputReportTypeFlags {
    fn write(&self, output: &mut BitVec<Msb0, u8>, _ctx: ()) -> Result<(), DekuError> {
        let byte = self.0.bits_c();
        output.extend([byte]);
        Ok(())
    }
}

#[bitflags]
#[repr(u16)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum GamepadButton {
    Nexus = 1 << 1,
    Menu = 1 << 2,
    View = 1 << 3,
    A = 1 << 4,
    B = 1 << 5,
    X = 1 << 6,
    Y = 1 << 7,
    DPadUp = 1 << 8,
    DPadDown = 1 << 9,
    DPadLeft = 1 << 10,
    DPadRight = 1 << 11,
    LeftShoulder = 1 << 12,
    RightShoulder = 1 << 13,
    LeftThumb = 1 << 14,
    RightThumb = 1 << 15,
}

/// Wrapper around input report type flags
#[derive(Debug, Eq, PartialEq)]
pub struct GamepadButtonFlags(BitFlags<GamepadButton>);

impl<'a> DekuRead<'a> for GamepadButtonFlags {
    fn read(
        input: &'a BitSlice<Msb0, u8>,
        _ctx: (),
    ) -> Result<(&'a BitSlice<Msb0, u8>, Self), DekuError>
    where
        Self: Sized,
    {
        let (rest, flags) = u16::read(input, ())?;
        let res = BitFlags::from_bits(flags)
            .map_err(|_| DekuError::Parse("Failed to read input report type flags".into()))?;

        Ok((rest, GamepadButtonFlags(res)))
    }
}

impl DekuWrite for GamepadButtonFlags {
    fn write(&self, output: &mut BitVec<Msb0, u8>, _ctx: ()) -> Result<(), DekuError> {
        // TODO: Verify little endian is correct
        output.extend(self.0.bits_c().to_le_bytes());
        Ok(())
    }
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
    pub button_mask: GamepadButtonFlags,
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
    report_type: InputReportTypeFlags,
    #[deku(cond = "!report_type.0.contains(InputReportType::Vibration)")]
    // Skip sequence info on vibration packets
    seq_info: Option<SequenceInfo>,
    #[deku(cond = "report_type.0.contains(InputReportType::Metadata)")]
    metadata_report: Option<MetadataReport>,
    #[deku(cond = "report_type.0.contains(InputReportType::GamepadReport)")]
    gamepad_report: Option<GamepadReport>,
    #[deku(cond = "report_type.0.contains(InputReportType::ClientMetadata)")]
    client_metadata_report: Option<ClientMetadataReport>,
    #[deku(cond = "report_type.0.contains(InputReportType::Vibration)")]
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
            let mut tmp_type: InputReportTypeFlags = InputReportTypeFlags::default();

            // Check which data will be contained
            if metadata_report.is_some() {
                tmp_type.0 |= InputReportType::Metadata;
            }
            if gamepad_report.is_some() {
                tmp_type.0 |= InputReportType::GamepadReport;
            }
            if client_metadata_report.is_some() {
                tmp_type.0 |= InputReportType::ClientMetadata;
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
        assert!(parsed.report_type.0.contains(InputReportType::Vibration));

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
}
