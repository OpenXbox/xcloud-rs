use std::sync::Arc;

use deku::{DekuContainerRead, DekuContainerWrite};
use tokio::time::{self, Instant, Interval};
use webrtc::data_channel::data_channel_message::DataChannelMessage;

use super::base::{DataChannelMsg, GssvChannel, GssvChannelSend};
use crate::{error::GsError, packets::input::{
    ClientMetadataReport, GamepadData, GamepadReport, InputMetadataEntry, InputPacket,
    MetadataReport,
}};

pub struct InputChannel {
    conn: Arc<webrtc::peer_connection::RTCPeerConnection>,
    inner: Arc<webrtc::data_channel::RTCDataChannel>,
    time_origin: Instant,
    input_sequence_num: u32,
    metadata_queue: Vec<InputMetadataEntry>,
    input_frames: Vec<GamepadData>,
    input_interval: Interval,
    rumble_enabled: bool,
}

impl GssvChannel for InputChannel {
    fn id() -> i32 {
        3
    }

    fn protocol() -> &'static str {
        "1.0"
    }

    fn is_ordered() -> Option<bool> {
        Some(true)
    }

    fn name() -> &'static str {
        "input"
    }

    fn new(peer_connection: Arc<webrtc::peer_connection::RTCPeerConnection>, inner: Arc<webrtc::data_channel::RTCDataChannel>) -> Self {
        Self {
            conn: peer_connection,
            inner: inner,
            time_origin: Instant::now(),
            input_sequence_num: 0,
            metadata_queue: vec![],
            input_frames: vec![],
            input_interval: time::interval(tokio::time::Duration::from_millis(100)),
            rumble_enabled: false
        }
    }

    fn conn(&self) -> Arc<webrtc::peer_connection::RTCPeerConnection> {
        self.conn.clone()
    }

    fn datachannel(&self) -> Arc<webrtc::data_channel::RTCDataChannel> {
        self.inner.clone()
    }

    async fn on_open(self: Arc<Self>) {
        log::warn!("TODO: Implement on_open for channel: '{}'", Self::name());
    }

    async fn on_close(self: Arc<Self>) {
        log::warn!("TODO: Implement on_close for channel: '{}'", Self::name());
    }

    async fn start(&self) -> Result<(), GsError> {
        let packet = InputPacket::new(
            //self.next_sequence_num(),
            0, // TODO
            // Fill timestamp
            self.timestamp(),
            None,
            None,
            Some(ClientMetadataReport::default()),
            None,
        );
        self.send_message(&DataChannelMsg::Bytes(packet.to_bytes().unwrap())).await
    }

    async fn on_message(self: Arc<Self>, msg: DataChannelMessage) {
        log::warn!("on_message (channel: {}): {:?}", Self::name(), msg);

        match msg.is_string {
            false => {
                let (_, input_packet) = InputPacket::from_bytes((&msg.data, 0)).unwrap();
                log::warn!("[{}] Received packet: {:?}", Self::name(), input_packet);
                // todo!("Handle input packet")
            }
            true => {
                let decoded = String::from_utf8_lossy(&msg.data.to_vec()).to_string();
                log::warn!("String message on InputChannel: {}", &decoded);
            }
        }
    }

    async fn on_error(self: Arc<Self>, error: webrtc::Error) {
        log::error!("Datachannel error, channel: {}, error: {error}", Self::name())
    }
}

impl InputChannel {
    fn next_sequence_num(&mut self) -> u32 {
        let current = self.input_sequence_num;
        self.input_sequence_num += 1;
        current
    }

    /// Get seconds since instantiation of this
    /// channel.
    fn timestamp(&self) -> f64 {
        self.time_origin.elapsed().as_secs_f64()
    }

    /// Handle incoming gamepad data.
    /// Stores the data into queue until drained
    /// by a call to `create_input_packet`
    fn on_button_press(&mut self, data: GamepadData) {
        println!("Received gamepad data");
        self.input_frames.push(data);
    }

    /// Create input packet containing gamepad data and
    /// metadata reports.
    /// This call will drain the respective queues.
    fn create_input_packet(&mut self) -> InputPacket {
        // Draining queues for metadata & gamepad data
        let gamepad_data: Vec<GamepadData> = self.input_frames.drain(..).collect();
        let metadata_reports: Vec<InputMetadataEntry> = self.metadata_queue.drain(..).collect();

        let gamepad_report = match gamepad_data.is_empty() {
            true => None,
            false => Some(GamepadReport {
                queue_len: gamepad_data.len() as u8,
                gamepad_data,
            }),
        };

        let metadata_report = match metadata_reports.is_empty() {
            true => None,
            false => Some(MetadataReport {
                queue_len: metadata_reports.len() as u8,
                metadata: metadata_reports,
            }),
        };

        InputPacket::new(
            self.next_sequence_num(),
            self.timestamp(),
            metadata_report,
            gamepad_report,
            None,
            None,
        )
    }

    /// Add processed input frame metadata to the queue.
    /// Queue will be drained by the next call to
    /// `create_input_packet`
    fn add_processed_frame(&mut self, metadata: InputMetadataEntry) {
        self.metadata_queue.push(metadata);
    }
}
