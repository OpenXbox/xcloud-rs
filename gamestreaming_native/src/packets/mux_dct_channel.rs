use crate::packets::{audio, input, qos, video};

/// Following channel classes exist:
///
/// Microsoft::Basix::Dct::Channel::Class::Audio
/// Microsoft::Basix::Dct::Channel::Class::Video
/// Microsoft::Basix::Dct::Channel::Class::Input
/// Microsoft::Basix::Dct::Channel::Class::InputV2
/// Microsoft::Basix::Dct::Channel::Class::Input Feedback
/// Microsoft::Basix::Dct::Channel::Class::ChatAudio
/// Microsoft::Basix::Dct::Channel::Class::Control
/// Microsoft::Basix::Dct::Channel::Class::Messaging
/// Microsoft::Basix::Dct::Channel::Class::QoS
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChannelType {
    Base,

    Audio,
    Video,
    Input,
    InputV2,
    InputFeedback,
    ChatAudio,
    Control,
    Messaging,
    QoS,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ChannelPacket {
    Audio(audio::AudioPacket),
    Video(video::VideoPacket),
    Input(input::InputPacket),
    Qos(qos::QosPacket),
}
