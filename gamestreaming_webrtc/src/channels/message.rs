use std::sync::Arc;

use crate::error::GsError;

use super::base::{DataChannelMsg, GssvChannel, GssvChannelSend};
use serde_json::{json, Value};
use tokio::sync::Mutex;
use webrtc::data_channel::data_channel_message::DataChannelMessage;

pub struct MessageChannel {
    conn: Arc<webrtc::peer_connection::RTCPeerConnection>,
    inner: Arc<webrtc::data_channel::RTCDataChannel>,
    handshake_ack_tx: tokio::sync::mpsc::UnboundedSender<()>,
    pub handshake_ack_rx: Arc<Mutex<tokio::sync::mpsc::UnboundedReceiver<()>>>,
}

impl GssvChannel for MessageChannel {
    fn id() -> i32 {
        5
    }

    fn protocol() -> &'static str {
        "messageV1"
    }

    fn is_ordered() -> Option<bool> {
        None
    }

    fn name() -> &'static str {
        "message"
    }

    fn new(peer_connection: Arc<webrtc::peer_connection::RTCPeerConnection>, inner: Arc<webrtc::data_channel::RTCDataChannel>) -> Self {
        let (handshake_ack_tx, handshake_ack_rx) = tokio::sync::mpsc::unbounded_channel();
        Self {
            conn: peer_connection,
            inner: inner,
            handshake_ack_tx,
            handshake_ack_rx: Arc::new(Mutex::new(handshake_ack_rx)),
        }
    }

    fn conn(&self) -> Arc<webrtc::peer_connection::RTCPeerConnection> {
        self.conn.clone()
    }

    fn datachannel(&self) -> Arc<webrtc::data_channel::RTCDataChannel> {
        self.inner.clone()
    }

    async fn on_open(self: Arc<Self>) {
        let handshake = json!({
            "type":"Handshake",
            "version":"messageV1",
            "id":"0ab125e2-6eee-4687-a2f4-5cfb347f0643",
            "cv":"",
        });
        self.send_message(&handshake.into()).await.unwrap();
    }

    async fn on_close(self: Arc<Self>) {
        log::warn!("TODO: Implement on_close for channel: '{}'", Self::name());
    }

    async fn start(&self) -> Result<(), GsError> {
        let auth_request = json!({
            "message":"authorizationRequest",
            "accessKey":"4BDB3609-C1F1-4195-9B37-FEFF45DA8B8E",
        });
        self.send_message(&auth_request.into()).await?;

        let gamepad_request = json!({
            "message": "gamepadChanged",
            "gamepadIndex": 0,
            "wasAdded": true,
        });
        self.send_message(&gamepad_request.into()).await
    }

    async fn on_message(self: Arc<Self>, msg: DataChannelMessage) {
        log::warn!("on_message (channel: {}): {:?}", Self::name(), msg);

        let json_msg: Value = serde_json::from_slice(&msg.data).unwrap();
        let msg_type = json_msg.get("type").unwrap().as_str().unwrap();
        match msg_type {
            "HandshakeAck" => {
                // Handshake has been acked.
                if let Err(err) = self.handshake_ack_tx.send(()) {
                    log::error!("Failed to submit handshake ack signal! error: {err}");
                }

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
                ).unwrap();
                self.send_message(&ui_config).await.unwrap();

                let client_config = Self::generate_message(
                    "/streaming/properties/clientappinstallidchanged",
                    &json!({ "clientAppInstallId": "4b8f472d-2c82-40e8-895d-bcd6a6ec7e9b" }),
                ).unwrap();
                self.send_message(&client_config).await.unwrap();

                let orientation_config = Self::generate_message(
                    "/streaming/characteristics/orientationchanged",
                    &json!({ "orientation": 0 }),
                ).unwrap();
                self.send_message(&orientation_config).await.unwrap();

                let touch_config = Self::generate_message(
                    "/streaming/characteristics/touchinputenabledchanged",
                    &json!({ "touchInputEnabled": /* self.getClient()._config.ui_touchenabled || */ false }),
                ).unwrap();
                self.send_message(&touch_config).await.unwrap();

                let device_config = Self::generate_message(
                    "/streaming/characteristics/clientdevicecapabilities",
                    &json!({}),
                ).unwrap();
                self.send_message(&device_config).await.unwrap();

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
                ).unwrap();
                self.send_message(&dimensions_config).await.unwrap();
            }
            "Message" => {
                log::warn!("Incoming Message: {:?}", json_msg);
            }
            val => {
                log::error!("[Channel: {}] Unhandled message type: {}", Self::name(), val);
            }
        }
    }

    async fn on_error(self: Arc<Self>, error: webrtc::Error) {
        log::error!("Datachannel error, channel: {}, error: {error}", Self::name())
    }
}

impl MessageChannel {
    fn generate_message(
        path: &str,
        data: &Value,
    ) -> Result<DataChannelMsg, GsError> {
        Ok(json!({
            "type": "Message",
            "content": serde_json::to_string(data).unwrap(),
            "id": "41f93d5a-900f-4d33-b7a1-2d4ca6747072",
            "target": path,
            "cv": "",
        })
        .into())
    }

    async fn send_transaction(&self, id: &str, data: &Value) -> Result<(), GsError> {
        let transaction = json!({
            "type": "TransactionComplete",
            "content": serde_json::to_string(data).unwrap(),
            "id": id,
            "cv": "",
        });

        self.send_message(&transaction.into()).await
    }
}
