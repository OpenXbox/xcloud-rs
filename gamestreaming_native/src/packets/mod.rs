mod audio;
mod input;
mod message;
mod mux_dct_channel;
mod mux_dct_control;
mod ping;
mod qos;
mod udp_connection_probing;
pub mod video;

use deku::prelude::*;
use hexdump;

use webrtc::rtp;

use mux_dct_control::MuxDCTControlHeader;
use udp_connection_probing::ConnectionProbingPacket;

#[derive(Debug, Clone, DekuRead, DekuWrite, PartialEq, Eq)]
#[deku(type = "u8")]
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

#[derive(Debug, Clone, DekuRead, DekuWrite, PartialEq, Eq)]
#[deku(type = "u8")]
pub enum ControlProtocolMessageOpCode {
    Auth = 0x1,
    AuthComplete = 0x2,
    Config = 0x3,
    ControllerChange = 0x4,
    Config2 = 0x6,
}

pub fn parse_rtp_packet(packet: &rtp::packet::Packet) {
    let (_, payload_type) =
        PayloadType::from_bytes((&packet.payload[..1], 0)).expect("Failed to parse PayloadType");

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
            println!(
                "RTP: {:?} Seq: {}, ts: {}, ssrc: {}",
                payload_type,
                packet.header.sequence_number,
                packet.header.timestamp,
                packet.header.ssrc
            );
            hexdump::hexdump(&packet.payload);
            let (_, packet) = MuxDCTControlHeader::from_bytes((&packet.payload[1..], 0))
                .expect("Failed to parse MuxDCTControlPacket");
            println!("{:?}", packet);
        }
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
            let (_, packet) = ConnectionProbingPacket::from_bytes((&packet.payload[1..], 0))
                .expect("Failed to parse UDPConnectionProbingPacket");

            println!("{:?}", packet);
        }
        /*
        PayloadType::URCPDummyPacket => {

        },
        PayloadType::MockUDPDctCtrl => {

        },
        */
        _ => {
            println!(
                "RTP: {:?} Seq: {}, ts: {}, ssrc: {}",
                payload_type,
                packet.header.sequence_number,
                packet.header.timestamp,
                packet.header.ssrc
            );
            hexdump::hexdump(&packet.payload);
        }
    }
}
