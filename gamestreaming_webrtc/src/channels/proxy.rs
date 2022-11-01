use super::{
    base::{
        ChannelExchangeMsg, ChannelType, DataChannelMsg, DataChannelParams, GssvChannel,
        GssvChannelProperties,
    },
    chat::ChatChannel,
    control::ControlChannel,
    input::InputChannel,
    message::MessageChannel,
};
use tokio::sync::mpsc;

#[derive(Debug)]
pub struct ChannelProxy {
    input: InputChannel,
    control: ControlChannel,
    message: MessageChannel,
    chat: ChatChannel,
    channel_to_client_mpsc: (
        mpsc::Sender<(ChannelType, ChannelExchangeMsg)>,
        mpsc::Receiver<(ChannelType, ChannelExchangeMsg)>,
    ),
}

impl ChannelProxy {
    pub fn data_channel_create_params() -> &'static [(ChannelType, DataChannelParams)] {
        &[
            (ChannelType::Input, InputChannel::PARAMS),
            (ChannelType::Control, ControlChannel::PARAMS),
            (ChannelType::Message, MessageChannel::PARAMS),
            (ChannelType::Chat, ChatChannel::PARAMS),
        ]
    }

    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(10);

        let mut this = Self {
            input: InputChannel::new(tx.clone()),
            control: ControlChannel::new(tx.clone()),
            message: MessageChannel::new(tx.clone()),
            chat: ChatChannel::new(tx.clone()),
            channel_to_client_mpsc: (tx, rx),
        };

        // Attach callback function
        this.message.on_handshake_ack(Box::new(move || {
            println!("Message channel received handshake ack, starting input/control messaging");
            this.input.start();
            this.control.start();

            Box::pin(async {})
        }));

        this
    }

    /// Used to receive messages from ChannelProxy in client
    pub fn get_receiver(self) -> mpsc::Receiver<(ChannelType, ChannelExchangeMsg)> {
        self.channel_to_client_mpsc.1
    }

    pub async fn handle_message(
        &mut self,
        typ: ChannelType,
        msg: ChannelExchangeMsg,
    ) -> Result<(), Box<dyn std::error::Error>> {
        /*
        let channel: Box<dyn GssvChannel> = match typ {
            ChannelType::Input => self.input,
            ChannelType::Control => self.control,
            ChannelType::Message => self.message,
            ChannelType::Chat => self.chat,
            _ => {
                return Err(format!("Unhandled channel type {:?}", typ));
            },
        };

        match msg {
            ChannelExchangeMsg::DataChannel(msg) => {
                channel.on_message(&msg).await
            },
            ChannelExchangeMsg::Event(evt) => {
                channel.on_event(&evt)
            }
        }
        */
        Ok(())
    }
}

impl Default for ChannelProxy {
    fn default() -> Self {
        Self::new()
    }
}
