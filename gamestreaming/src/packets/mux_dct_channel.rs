use crate::packets::{audio, video, input, qos};

#[derive(Debug, Clone, PartialEq)]
pub enum ChannelType {
    Base,

    Audio,
    Video,
    Input,
    QoS,
    Control
}

#[derive(Debug, Clone, PartialEq)]
pub enum ChannelPacket {
    Audio(audio::AudioPacket),
    Video(video::VideoPacket),
    Input(input::InputPacket),
    Qos(qos::QosPacket),
}