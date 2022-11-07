use gilrs::{
    Button, EventType, Axis,
    ff::{Effect, EffectBuilder, BaseEffect, BaseEffectType, Replay, Ticks}
};
use crate::{GamepadData, packets::input::VibrationReport};

pub struct GamepadProcessor {
    state: GamepadData,
}

impl GamepadProcessor {
    pub fn new() -> Self {
        Self {
            state: GamepadData::default(),
        }
    }

    pub fn get_data(&self) -> GamepadData {
        self.state
    }

    pub fn add_event(&mut self, event: EventType) {
        match event {
            EventType::ButtonPressed(btn, _) => {
                let set_to = true;
                match btn {
                    Button::South => self.state.button_mask.A = set_to,
                    Button::East => self.state.button_mask.B = set_to,
                    Button::North => self.state.button_mask.Y = set_to,
                    Button::West => self.state.button_mask.X = set_to,
                    Button::LeftTrigger => self.state.button_mask.LeftShoulder = set_to,
                    Button::RightTrigger => self.state.button_mask.RightShoulder = set_to,
                    Button::Select => self.state.button_mask.View = set_to,
                    Button::Start => self.state.button_mask.Menu = set_to,
                    Button::Mode => self.state.button_mask.Nexus = set_to,
                    Button::LeftThumb => self.state.button_mask.LeftThumb = set_to,
                    Button::RightThumb => self.state.button_mask.RightThumb = set_to,
                    Button::DPadUp => self.state.button_mask.DPadUp = set_to,
                    Button::DPadDown => self.state.button_mask.DPadDown = set_to,
                    Button::DPadLeft => self.state.button_mask.DPadLeft = set_to,
                    Button::DPadRight => self.state.button_mask.DPadRight = set_to,
                    Button::Unknown => {
                        eprintln!("Unknown button pressed");
                    },
                    val => {
                        eprintln!("Unhandled button pressed: {:?}", val);
                    }
                }
            }
            EventType::ButtonReleased(btn, _) => {
                let set_to = false;
                match btn {
                    Button::South => self.state.button_mask.A = set_to,
                    Button::East => self.state.button_mask.B = set_to,
                    Button::North => self.state.button_mask.Y = set_to,
                    Button::West => self.state.button_mask.X = set_to,
                    Button::LeftTrigger => self.state.button_mask.LeftShoulder = set_to,
                    Button::RightTrigger => self.state.button_mask.RightShoulder = set_to,
                    Button::Select => self.state.button_mask.View = set_to,
                    Button::Start => self.state.button_mask.Menu = set_to,
                    Button::Mode => self.state.button_mask.Nexus = set_to,
                    Button::LeftThumb => self.state.button_mask.LeftThumb = set_to,
                    Button::RightThumb => self.state.button_mask.RightThumb = set_to,
                    Button::DPadUp => self.state.button_mask.DPadUp = set_to,
                    Button::DPadDown => self.state.button_mask.DPadDown = set_to,
                    Button::DPadLeft => self.state.button_mask.DPadLeft = set_to,
                    Button::DPadRight => self.state.button_mask.DPadRight = set_to,
                    Button::Unknown => {
                        eprintln!("Unknown button released");
                    },
                    val => {
                        eprintln!("Unhandled button released: {:?}", val);
                    }
                }
            }
            EventType::AxisChanged(axis, val, _) => {
                let val_i16 = (val * (i16::MAX as f32)) as i16;
                let val_u16 = (val * (u16::MAX as f32)) as u16;
                match axis {
                    Axis::LeftStickX => { self.state.left_thumb_x = val_i16 }
                    Axis::LeftStickY => { self.state.left_thumb_y = val_i16 }
                    Axis::RightStickX => { self.state.right_thumb_x = val_i16 }
                    Axis::RightStickY => { self.state.right_thumb_y = val_i16 }
                    Axis::LeftZ => { self.state.left_trigger = val_u16 }
                    Axis::RightZ => { self.state.right_trigger = val_u16 }
                    Axis::DPadX | Axis::DPadY | Axis::Unknown => {
                        eprintln!("Unhandled axis changed: {:?}", axis);
                    }
                }
            }
            EventType::Connected => {
                eprintln!("Controller connected");
            },
            EventType::Disconnected => {
                eprintln!("Controller disconnected");
            },
            EventType::ButtonRepeated(..)
            | EventType::ButtonChanged(..)
            | EventType::Dropped => {},
        }
    }
}

impl Default for GamepadProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl From<VibrationReport> for BaseEffect {
    fn from(report: VibrationReport) -> Self {
        BaseEffect {
            kind: BaseEffectType::Strong { magnitude: 60_000 },
            scheduling: Replay {
                after: Ticks::from_ms(50),
                play_for: Ticks::from_ms(report.duration_ms.into()),
                with_delay: Ticks::from_ms(report.delay_ms.into()),
            },
            ..Default::default()
        }
    }
}
