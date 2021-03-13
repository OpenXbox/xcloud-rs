mod serializing;
mod udp_connection_probing;
mod mux_dct_control;
mod mux_dct_channel;
mod audio;
mod video;
mod input;
mod qos;
mod message;


use std::convert::{Into, From};
use std::io::{Cursor};
use hexdump;

use crate::webrtc::rtp;

use serializing::{Deserialize};
use udp_connection_probing::{ConnectionProbingPacket, ConnectionProbingType, ConnectionProbingSyn, ConnectionProbingAck};
use mux_dct_control::MuxDCTControlPacket;

#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum PayloadType {
    Unknown = 0x0,
    MuxDCTChannelRangeDefault = 0x23,
    MuxDCTChannelRangeEnd = 0x3f,
    BaseLinkControl = 0x60,
    MuxDCTControl = 0x61,
    FECControl = 0x62,
    SecurityLayerCtrl = 0x63,
    URCPControl = 0x64,
    UDPKeepAlive = 0x65,
    UDPConnectionProbing = 0x66,
    URCPDummyPacket = 0x68,
    MockUDPDctCtrl = 0x7f,
}

impl From<u8> for PayloadType {
    fn from(value: u8) -> Self {
        let z: PayloadType = unsafe { ::std::mem::transmute(value) };

        z
    }
}

#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum ControlProtocolMessageOpCode {
    Auth = 0x1,
    AuthComplete = 0x2,
    Config = 0x3,
    ControllerChange = 0x4,
    Config2 = 0x6,
}

impl From<u8> for ControlProtocolMessageOpCode {
    fn from(value: u8) -> Self {
        let z: ControlProtocolMessageOpCode = unsafe { ::std::mem::transmute(value) };

        z
    }
}

pub fn parse_rtp_packet(packet: &rtp::packet::Packet) {
    let payload_type: PayloadType = packet.header.payload_type.into();
    let mut reader = Cursor::new(&packet.payload);

    match payload_type {
        /*
        PayloadType::MuxDCTChannelRangeDefault => {

        },
        PayloadType::MuxDCTChannelRangeEnd => {

        },
        PayloadType::BaseLinkControl => {

        },
        */
        PayloadType::MuxDCTControl => {
            println!("RTP: {:?} Seq: {}, ts: {}, ssrc: {}",
                payload_type,
                packet.header.sequence_number,
                packet.header.timestamp,
                packet.header.ssrc
            );
            hexdump::hexdump(&packet.payload);
            let packet = MuxDCTControlPacket::deserialize(&mut reader)
                .expect("Failed to parse MuxDCTControlPacket");
            println!("{:?}", packet);
        },
        /*
        PayloadType::FECControl => {

        },
        PayloadType::SecurityLayerCtrl => {

        },
        PayloadType::URCPControl => {
        },
        PayloadType::UDPKeepAlive => {
        },
        */
        PayloadType::UDPConnectionProbing => {
            let packet = ConnectionProbingPacket::deserialize(&mut reader)
                .expect("Failed to parse UDPConnectionProbingPacket");

            match packet {
                ConnectionProbingPacket::Syn(pdata) => {
                    println!("ConnectionProbingPacket::Syn(DataLen={})", pdata.probe_data.len());
                },
                ConnectionProbingPacket::Ack(pdata) => {
                    println!("ConnectionProbingPacket::Ack(AcceptedSize={}, Appendix={})", pdata.accepted_packet_size, pdata.appendix);
                }
            }
        },
        /*
        PayloadType::URCPDummyPacket => {

        },
        PayloadType::MockUDPDctCtrl => {

        },
        */
        _ => {
            println!("RTP: {:?} Seq: {}, ts: {}, ssrc: {}",
                payload_type,
                packet.header.sequence_number,
                packet.header.timestamp,
                packet.header.ssrc
            );
            hexdump::hexdump(&packet.payload);
        }
    }
}
