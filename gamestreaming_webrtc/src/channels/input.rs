use deku::{DekuContainerRead, DekuContainerWrite};
use tokio::time::{Instant, Interval};

use super::base::{DataChannelMsg, GssvChannel, GssvChannelEvent};
use crate::packets::input::{
    ClientMetadataReport, GamepadData, GamepadReport, InputMetadataEntry, InputPacket,
    MetadataReport,
};

pub struct InputChannel {
    time_origin: Instant,
    input_sequence_num: u32,
    metadata_queue: Vec<InputMetadataEntry>,
    input_frames: Vec<GamepadData>,
    input_interval: Interval,
    rumble_enabled: bool,
}

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

    fn start(&mut self) {
        let packet = InputPacket::new(
            self.next_sequence_num(),
            // Fill timestamp
            self.timestamp(),
            None,
            None,
            Some(ClientMetadataReport::default()),
        );
        self.send_message(&DataChannelMsg::Bytes(packet.to_bytes().unwrap()));
    }

    fn on_message(&self, msg: &DataChannelMsg) -> Result<(), Box<dyn std::error::Error>> {
        println!("on_message ({}): {:?}", Self::name(), msg);

        match msg {
            DataChannelMsg::Bytes(bytes) => {
                let (_, input_packet) = InputPacket::from_bytes((bytes, 0))?;
                println!("[{}] Received packet: {:?}", Self::name(), input_packet);
                todo!("Handle input packet")
            }
            val => {
                Err(format!("[{}] Unhandled message type: {:?}", Self::name(), val).into())
            }
        }
    }

    fn send_message(&self, msg: &DataChannelMsg) {
        todo!()
    }

    fn send_event(&self, event: &GssvChannelEvent) {
        todo!()
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
        )
    }

    /// Add processed input frame metadata to the queue.
    /// Queue will be drained by the next call to
    /// `create_input_packet`
    fn add_processed_frame(&mut self, metadata: InputMetadataEntry) {
        self.metadata_queue.push(metadata);
    }
}
