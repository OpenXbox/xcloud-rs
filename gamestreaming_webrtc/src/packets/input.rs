use deku::{
    bitvec::{BitSlice, BitVec, Msb0},
    prelude::*,
};
use enumflags2::{bitflags, BitFlags};

#[bitflags]
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq)]
enum InputReportType {
    Metadata = 1,
    GamepadReport = 2,
    ClientMetadata = 8,
    ServerMetadata = 16,
    Mouse = 32,
    Keyboard = 64,
    Vibration = 128,
}

/// Wrapper around input report type flags
#[derive(Debug, PartialEq)]
struct InputReportTypeFlags(BitFlags<InputReportType>);

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

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
struct VibrationReport {
    /// Rumble Type: 0 = FourMotorRumble
    rumble_type: u8,
    gamepad_id: u8,

    left_motor_percent: u8,
    right_motor_percent: u8,
    left_trigger_motor_percent: u8,
    right_trigger_motor_percent: u8,
    duration_ms: u16,
    delay_ms: u16,
    repeat: u8,
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
struct InputMetadataEntry {
    server_data_key: u32,
    first_frame_packet_arrival_time_ms: u32,
    frame_submitted_time_ms: u32,
    frame_decoded_time_ms: u32,
    frame_rendered_time_ms: u32,
    frame_packet_time: u32,
    frame_date_now: u32,
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
struct MetadataReport {
    #[deku(update = "self.metadata.len()")]
    queue_len: u8,
    #[deku(count = "queue_len")]
    metadata: Vec<InputMetadataEntry>,
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
struct GamepadData {
    gamepad_index: u8,
    button_mask: u16,
    left_thumb_x: i16,
    left_thumb_y: i16,
    right_thumb_x: i16,
    right_thumb_y: i16,
    left_trigger: u16,
    right_trigger: u16,
    physical_physicality: u32,
    virtual_physicality: u32,
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
struct GamepadReport {
    #[deku(update = "self.gamepad_data.len()")]
    queue_len: u8,
    #[deku(count = "queue_len")]
    gamepad_data: Vec<GamepadData>,
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
struct ClientMetadataReport {
    metadata: u8,
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
struct SequenceInfo {
    sequence_num: u32,
    timestamp: f64,
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
struct InputPacket {
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
