use super::error::FlagsError;
use deku::prelude::*;
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

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "little")]
struct InputRumblePacket {
    report_type: u8,
    /// Rumble Type: 0 = FourMotorRumble
    rumble_type: u8,
    _unknown: u8,

    left_motor_percent: u8,
    right_motor_percent: u8,
    left_trigger_motor_percent: u8,
    right_trigger_motor_percent: u8,
    duration_ms: u16,
    delay_ms: u16,
    repeat: u8,
}

impl InputRumblePacket {
    fn get_report_type(&self) -> Result<BitFlags<InputReportType>, FlagsError<InputReportType>> {
        BitFlags::from_bits(self.report_type).map_err(FlagsError::DeserializeError)
    }

    fn set_report_type(&mut self, report_type: BitFlags<InputReportType>) {
        self.report_type = BitFlags::bits(report_type);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_input_rumble_packet() {
        let test_data = vec![
            0x80, 0x00, 0x00, 0xF1, 0xF2, 0xF3, 0xF4, 0x50, 0x01, 0xFF, 0x01, 0x10,
        ];
        let (rest, parsed) = InputRumblePacket::from_bytes((&test_data, 0))
            .expect("Failed to deserialize input rumble packet");

        println!("{:?}", rest);
        assert!(rest.0.is_empty());
        assert_eq!(rest.1, 0);
        assert_eq!(parsed.report_type, 0x80);
        assert_eq!(parsed.rumble_type, 0x00);
        assert_eq!(parsed._unknown, 0x00);
        assert_eq!(parsed.left_motor_percent, 0xF1);
        assert_eq!(parsed.right_motor_percent, 0xF2);
        assert_eq!(parsed.left_trigger_motor_percent, 0xF3);
        assert_eq!(parsed.right_trigger_motor_percent, 0xF4);
        assert_eq!(parsed.duration_ms, 0x150);
        assert_eq!(parsed.delay_ms, 0x1FF);
        assert_eq!(parsed.repeat, 0x10);

        let report_type = parsed
            .get_report_type()
            .expect("Failed to get input report type flags");
        assert!(report_type.contains(InputReportType::Vibration));
        assert!(!report_type.contains(InputReportType::Mouse));
    }
}
