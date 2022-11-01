use std::{pin::Pin, future::Future, sync::{Arc}};
use tokio::sync::Mutex;

use super::base::{
    ChannelExchangeMsg, ChannelType, DataChannelMsg, DataChannelParams, GssvChannel,
    GssvChannelProperties,
};
use async_trait::async_trait;
use serde_json::{json, Value};
use tokio::sync::mpsc;

pub type OnHandshakeAckHdlrFn = Box<dyn (FnOnce() -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>) + Send + Sync>;

pub struct MessageChannel {
    sender: mpsc::Sender<(ChannelType, ChannelExchangeMsg)>,
    on_handshake_ack_handler: Arc<Mutex<Option<OnHandshakeAckHdlrFn>>>,
}

impl GssvChannelProperties for MessageChannel {
    const TYPE: ChannelType = ChannelType::Message;
    const PARAMS: DataChannelParams = DataChannelParams {
        id: 5,
        protocol: "messageV1",
        is_ordered: None,
    };
    fn sender(&self) -> &mpsc::Sender<(ChannelType, ChannelExchangeMsg)> {
        &self.sender
    }
}

#[async_trait]
impl GssvChannel for MessageChannel {
    async fn on_open(&self) -> Result<(), Box<dyn std::error::Error>> {
        let handshake = json!({
            "type":"Handshake",
            "version":"messageV1",
            "id":"0ab125e2-6eee-4687-a2f4-5cfb347f0643",
            "cv":"",
        });
        self.send_message(handshake.into()).await
    }

    async fn on_close(&self) -> Result<(), Box<dyn std::error::Error>> {
        todo!()
    }

    async fn on_message(&self, msg: &DataChannelMsg) -> Result<(), Box<dyn std::error::Error>> {
        println!("on_message ({:?}): {:?}", Self::TYPE, msg);

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
                self.send_message(ui_config).await?;

                let client_config = Self::generate_message(
                    "/streaming/properties/clientappinstallidchanged",
                    &json!({ "clientAppInstallId": "4b8f472d-2c82-40e8-895d-bcd6a6ec7e9b" }),
                )?;
                self.send_message(client_config).await?;

                let orientation_config = Self::generate_message(
                    "/streaming/characteristics/orientationchanged",
                    &json!({ "orientation": 0 }),
                )?;
                self.send_message(orientation_config).await?;

                let touch_config = Self::generate_message(
                    "/streaming/characteristics/touchinputenabledchanged",
                    &json!({ "touchInputEnabled": /* self.getClient()._config.ui_touchenabled || */ false }),
                )?;
                self.send_message(touch_config).await?;

                let device_config = Self::generate_message(
                    "/streaming/characteristics/clientdevicecapabilities",
                    &json!({}),
                )?;
                self.send_message(device_config).await?;

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
                self.send_message(dimensions_config).await?;
            }
            val => {
                return Err(format!("[{:?}] Unhandled message type: {}", Self::TYPE, val).into());
            }
        };

        Ok(())
    }
}

impl MessageChannel {
    pub fn new(sender: mpsc::Sender<(ChannelType, ChannelExchangeMsg)>) -> Self {
        Self {
            sender,
            on_handshake_ack_handler: Default::default(),
        }
    }

    pub(crate) async fn on_handshake_ack(&self, f: OnHandshakeAckHdlrFn) {
        let mut handler = self.on_handshake_ack_handler.lock().await;
        *handler = Some(f);
    }

    async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        let auth_request = json!({
            "message":"authorizationRequest",
            "accessKey":"4BDB3609-C1F1-4195-9B37-FEFF45DA8B8E",
        });
        self.send_message(auth_request.into()).await?;

        let gamepad_request = json!({
            "message": "gamepadChanged",
            "gamepadIndex": 0,
            "wasAdded": true,
        });
        self.send_message(gamepad_request.into()).await
    }

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

    async fn send_transaction(
        &self,
        id: &str,
        data: &Value,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let transaction = json!({
            "type": "TransactionComplete",
            "content": serde_json::to_string(data)?,
            "id": id,
            "cv": "",
        });

        self.send_message(transaction.into()).await
    }
}

impl std::fmt::Debug for MessageChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MessageChannel")
            .field("sender", &self.sender)
            .field("on_handshake_ack_handler", &"<>")
            .finish()
    }
}