use std::sync::Arc;
use webrtc::data_channel::{data_channel_init::RTCDataChannelInit, data_channel_message::DataChannelMessage};
use crate::error::GsError;

#[derive(Debug)]
pub enum ChannelType {
    Chat,
    Control,
    Input,
    Message,
    Audio,
    Video,
}

#[derive(Debug)]
pub struct GssvChannelEvent(String);

#[derive(Debug)]
pub enum DataChannelMsg {
    String(String),
    Bytes(Vec<u8>),
}

impl From<serde_json::Value> for DataChannelMsg {
    fn from(val: serde_json::Value) -> Self {
        let str =
            serde_json::to_string(&val).expect("Failed to serialize message for DataChannelMsg");
        DataChannelMsg::String(str)
    }
}

impl From<DataChannelMessage> for DataChannelMsg {
    fn from(value: DataChannelMessage) -> Self {
        match value.is_string {
            true => Self::String(String::from_utf8_lossy(&value.data).to_string()),
            false => Self::Bytes(value.data.to_vec())
        }
    }
}

impl TryFrom<&DataChannelMsg> for serde_json::Value {
    type Error = Box<dyn std::error::Error>;

    fn try_from(value: &DataChannelMsg) -> Result<Self, Self::Error> {
        match value {
            DataChannelMsg::String(str) => serde_json::from_str(str).map_err(|e| e.into()),
            _ => Err("Can only convert DataChannelMsg::String to JSON".into()),
        }
    }
}

pub trait GssvChannelInit {
    fn init(conn: Arc<webrtc::peer_connection::RTCPeerConnection>) -> impl std::future::Future<Output = Result<Arc<Self>, GsError>> + Send + Sync
        where Self: Sized;
}

pub trait GssvChannelSend {
    fn send_message(&self, msg: &DataChannelMsg) -> impl std::future::Future<Output = Result<(), GsError>> + Send;
    fn send_event(&self, event: &GssvChannelEvent) -> impl std::future::Future<Output = Result<(), GsError>> + Send;
}

pub trait GssvChannel {
    fn id() -> i32;
    fn protocol() -> &'static str;
    fn is_ordered() -> Option<bool>;
    fn name() -> &'static str;
    fn new(peer_connection: Arc<webrtc::peer_connection::RTCPeerConnection>, inner: Arc<webrtc::data_channel::RTCDataChannel>) -> Self;
    fn conn(&self) -> Arc<webrtc::peer_connection::RTCPeerConnection>;
    fn datachannel(&self) -> Arc<webrtc::data_channel::RTCDataChannel>;

    fn on_open(self: Arc<Self>) -> impl std::future::Future<Output = ()> + Send;
    fn on_close(self: Arc<Self>) -> impl std::future::Future<Output = ()> + Send;
    fn on_error(self: Arc<Self>, error: webrtc::Error) -> impl std::future::Future<Output = ()> + Send;
    fn on_message(self: Arc<Self>, msg: DataChannelMessage) -> impl std::future::Future<Output = ()> + Send;

    fn start(&self) -> impl std::future::Future<Output = Result<(), GsError>> + Send;
}

impl<T> GssvChannelInit for T
    where T: GssvChannel + Send + Sync + 'static
{
    async fn init(conn: Arc<webrtc::peer_connection::RTCPeerConnection>) -> Result<Arc<Self>, GsError> {
        use crate::channels::weak_callback::WeakAsyncCallback;
        let chan = conn
            .create_data_channel(
                T::name(),
                Some(RTCDataChannelInit {
                    ordered: T::is_ordered(),
                    protocol: Some(T::protocol().to_owned()),
                    ..Default::default()
                }),
            )
            .await
            .map_err(|_|GsError::DataChannelInit("Data channel creation failed".into()))?;

        let instance = Arc::new(T::new(conn, chan.clone()));

        // Assign callbacks
        chan.on_message(Box::with_weak_async_callback(&instance, T::on_message));
        chan.on_open(Box::with_weak_async_callback(&instance, T::on_open));
        chan.on_close(Box::with_weak_async_callback(&instance, T::on_close));
        chan.on_error(Box::with_weak_async_callback(&instance, T::on_error));

        Ok(instance)
    }
}

impl<T> GssvChannelSend for T
    where T: GssvChannel + Send + Sync + 'static
{
    async fn send_message(&self, msg: &DataChannelMsg) -> Result<(), GsError> {
        let channel = self.datachannel();

        let sent = match msg {
            DataChannelMsg::String(string_data) => channel.send_text(string_data).await,
            DataChannelMsg::Bytes(binary_data) => channel.send(&bytes::Bytes::copy_from_slice(&binary_data)).await,
        }
        .map_err(|e|GsError::DataChannelInit(format!("Failed to send message on channel: {}, error: {}", T::name(), e)))?;

        log::info!("DataChannel {} sent message of {} bytes", T::name(), sent);

        Ok(())
    }

    async fn send_event(&self, _event: &GssvChannelEvent) -> Result<(), GsError> {
        todo!()
    }
}
