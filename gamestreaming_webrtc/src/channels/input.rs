use super::base::{DataChannelMsg, GssvChannel, GssvChannelEvent};
pub struct InputChannel;

impl GssvChannel for InputChannel {
    fn name() -> &'static str {
        "Input"
    }

    fn on_open(&self) {
        todo!()
    }

    fn on_close(&self) {
        todo!()
    }

    fn start(&self) {}

    fn send_message(&self, msg: &DataChannelMsg) {
        todo!()
    }

    fn send_event(&self, event: &GssvChannelEvent) {
        todo!()
    }
}

impl InputChannel {}
