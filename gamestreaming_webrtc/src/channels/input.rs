use std::default::Default;

use async_trait::async_trait;
use deku::{DekuContainerRead, DekuContainerWrite};
use tokio::{sync::mpsc, time::Instant};

use super::base::{
    ChannelExchangeMsg, ChannelType, DataChannelMsg, DataChannelParams, GssvChannel,
    GssvChannelProperties,
};
use crate::{packets::input::{
    ClientMetadataReport, GamepadData, GamepadReport, InputMetadataEntry, InputPacket,
    MetadataReport,
}, GssvChannelEvent};

#[derive(Debug)]
pub struct InputChannel {
    time_origin: Instant,
    input_sequence_num: u32,
    metadata_queue: Vec<InputMetadataEntry>,
    input_frames: Vec<GamepadData>,
    rumble_enabled: bool,
    sender: mpsc::Sender<(ChannelType, ChannelExchangeMsg)>,
}

impl GssvChannelProperties for InputChannel {
    const TYPE: ChannelType = ChannelType::Input;
    const PARAMS: DataChannelParams = DataChannelParams {
        id: 3,
        protocol: "1.0",
        is_ordered: Some(true),
    };
    fn sender(&self) -> &mpsc::Sender<(ChannelType, ChannelExchangeMsg)> {
        &self.sender
    }
}

#[async_trait]
impl GssvChannel for InputChannel {
    async fn on_message(&self, msg: &DataChannelMsg) -> Result<(), Box<dyn std::error::Error>> {
        println!("on_message ({:?}): {:?}", Self::TYPE, msg);

        match msg {
            DataChannelMsg::Bytes(bytes) => {
                let (_, input_packet) = InputPacket::from_bytes((bytes, 0))?;
                println!("[{:?}] Received packet: {:?}", Self::TYPE, input_packet);
                if let Some(vibration) = input_packet.vibration_report {
                    // Pass back the rumble description to the client
                    self.send_event(GssvChannelEvent::GamepadRumble(vibration));
                }
                Ok(())
            }
            val => Err(format!("[{:?}] Unhandled message type: {:?}", Self::TYPE, val).into()),
        }
    }
}

impl InputChannel {
    pub fn new(sender: mpsc::Sender<(ChannelType, ChannelExchangeMsg)>) -> Self {
        Self {
            sender,
            time_origin: Instant::now(),
            input_sequence_num: 0,
            metadata_queue: vec![],
            input_frames: vec![],
            rumble_enabled: true,
        }
    }

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

    pub(crate) async fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let packet = InputPacket::new(
            self.next_sequence_num(),
            // Fill timestamp
            self.timestamp(),
            None,
            None,
            Some(ClientMetadataReport::default()),
        );
        self.send_message(DataChannelMsg::Bytes(packet.to_bytes().unwrap()))
            .await
    }

    /// Handle incoming gamepad data.
    /// Stores the data into queue until drained
    /// by a call to `create_input_packet`
    pub async fn on_button_press(&mut self, data: &GamepadData) -> Result<(), Box<dyn std::error::Error>> {
        println!("Received gamepad data");
        self.input_frames.push(*data);

        // TODO: Call this somewhere else
        let pkt = self.create_input_packet().to_bytes().unwrap();
        self.send_message(DataChannelMsg::Bytes(pkt)).await
    }

    pub async fn on_metadata(&mut self, data: &InputMetadataEntry) -> Result<(), Box<dyn std::error::Error>> {
        println!("Received gamepad data");
        self.metadata_queue.push(*data);
        Ok(())
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
        )
    }

    /// Add processed input frame metadata to the queue.
    /// Queue will be drained by the next call to
    /// `create_input_packet`
    fn add_processed_frame(&mut self, metadata: InputMetadataEntry) {
        self.metadata_queue.push(metadata);
    }
}
