use super::base::{DataChannelMsg, GssvChannel, GssvChannelEvent};
pub struct ChatChannel;

impl GssvChannel for ChatChannel {
    fn name() -> &'static str {
        "Chat"
    }

    fn on_open(&self) {
        todo!()
    }

    fn on_close(&self) {
        todo!()
    }

    fn start(&mut self) {
        todo!()
    }

    fn send_message(&self, msg: &DataChannelMsg) {
        todo!()
    }

    fn send_event(&self, event: &GssvChannelEvent) {
        todo!()
    }
}
