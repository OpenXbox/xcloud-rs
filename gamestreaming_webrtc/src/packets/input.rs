use deku::prelude::*;

enum InputReportType {
    None = 0,
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

#[cfg(test)]
mod tests {}
