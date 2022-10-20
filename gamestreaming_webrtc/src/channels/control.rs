use super::base::{DataChannelMsg, GssvChannel, GssvChannelEvent};
use serde_json::json;

pub struct ControlChannel;

impl GssvChannel for ControlChannel {
    fn name() -> &'static str {
        "Control"
    }

    fn on_open(&self) {
        todo!()
    }

    fn on_close(&self) {
        todo!()
    }

    fn start(&mut self) {
        let auth_request = json!({
            "message":"authorizationRequest",
            "accessKey":"4BDB3609-C1F1-4195-9B37-FEFF45DA8B8E",
        });
        self.send_message(&auth_request.into());

        let gamepad_request = json!({
            "message": "gamepadChanged",
            "gamepadIndex": 0,
            "wasAdded": true,
        });
        self.send_message(&gamepad_request.into())
    }

    fn send_message(&self, msg: &DataChannelMsg) {
        todo!()
    }

    fn send_event(&self, event: &GssvChannelEvent) {
        todo!()
    }
}

impl ControlChannel {
    fn request_keyframe(&self) {
        let keyframe_request = json!({
            "message": "videoKeyframeRequested",
            "ifrRequested": true,
        });

        self.send_message(&keyframe_request.into())
    }
}
