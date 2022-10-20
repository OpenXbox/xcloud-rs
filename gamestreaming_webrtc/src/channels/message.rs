use super::base::{DataChannelMsg, GssvChannel, GssvChannelEvent};
use serde_json::{json, Value};

pub struct MessageChannel;

impl GssvChannel for MessageChannel {
    fn name() -> &'static str {
        "Message"
    }

    fn on_open(&self) {
        let handshake = json!({
            "type":"Handshake",
            "version":"messageV1",
            "id":"0ab125e2-6eee-4687-a2f4-5cfb347f0643",
            "cv":"",
        });
        self.send_message(&handshake.into())
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

    fn on_message(&self, msg: &DataChannelMsg) -> Result<(), Box<dyn std::error::Error>> {
        println!("on_message ({}): {:?}", Self::name(), msg);

        let json_msg: Value = msg.try_into()?;
        let msg_type = json_msg.get("type").unwrap().as_str().unwrap();
        match msg_type {
            "HandshakeAck" => {
                // Handshake has been acked.

                //self.getClient().getChannelProcessor("control").start()
                //self.getClient().getChannelProcessor("input").start()

                let system_uis = /* self.getClient()._config.ui_systemui || */ [10, 19, 31, 27, 32, -41];
                let system_version = /* self.getClient()._config.ui_version || */ [0, 1, 0];
                let ui_config = Self::generate_message(
                    "/streaming/systemUi/configuration",
                    &json!({
                        "version": system_version,
                        "systemUis": system_uis, // Xbox Windows app has [33], xCloud has [10,19,31,27,32,-41]

                        // 10 = ShowVirtualKeyboard
                        // 19 = ShowMessageDialog
                        // 31 = ShowApplication
                        // 27 = ShowPurchase
                        // 32 = ShowTimerExtensions
                        // 33 = Xbox windows app, disables the nexus menu on xCloud (Alt nexus menu?)
                        // -41 = unknown
                        // Possible options: Keyboard, PurchaseModal
                    }),
                )?;
                self.send_message(&ui_config);

                let client_config = Self::generate_message(
                    "/streaming/properties/clientappinstallidchanged",
                    &json!({ "clientAppInstallId": "4b8f472d-2c82-40e8-895d-bcd6a6ec7e9b" }),
                )?;
                self.send_message(&client_config);

                let orientation_config = Self::generate_message(
                    "/streaming/characteristics/orientationchanged",
                    &json!({ "orientation": 0 }),
                )?;
                self.send_message(&orientation_config);

                let touch_config = Self::generate_message(
                    "/streaming/characteristics/touchinputenabledchanged",
                    &json!({ "touchInputEnabled": /* self.getClient()._config.ui_touchenabled || */ false }),
                )?;
                self.send_message(&touch_config);

                let device_config = Self::generate_message(
                    "/streaming/characteristics/clientdevicecapabilities",
                    &json!({}),
                )?;
                self.send_message(&device_config);

                let dimensions_config = Self::generate_message(
                    "/streaming/characteristics/dimensionschanged",
                    &json!({
                        "horizontal": 1920,
                        "vertical": 1080,
                        "preferredWidth": 1920,
                        "preferredHeight": 1080,
                        "safeAreaLeft": 0,
                        "safeAreaTop": 0,
                        "safeAreaRight": 1920,
                        "safeAreaBottom": 1080,
                        "supportsCustomResolution":true,
                    }),
                )?;
                self.send_message(&dimensions_config);
            }
            val => {
                return Err(format!("[{}] Unhandled message type: {}", Self::name(), val).into());
            }
        };

        Ok(())
    }

    fn send_message(&self, msg: &DataChannelMsg) {
        todo!()
    }

    fn send_event(&self, event: &GssvChannelEvent) {
        todo!()
    }
}

impl MessageChannel {
    fn generate_message(
        path: &str,
        data: &Value,
    ) -> Result<DataChannelMsg, Box<dyn std::error::Error>> {
        Ok(json!({
            "type": "Message",
            "content": serde_json::to_string(data)?,
            "id": "41f93d5a-900f-4d33-b7a1-2d4ca6747072",
            "target": path,
            "cv": "",
        })
        .into())
    }

    fn send_transaction(&self, id: &str, data: &Value) -> Result<(), Box<dyn std::error::Error>> {
        let transaction = json!({
            "type": "TransactionComplete",
            "content": serde_json::to_string(data)?,
            "id": id,
            "cv": "",
        });

        self.send_message(&transaction.into());
        Ok(())
    }
}
