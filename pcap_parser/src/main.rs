/// Based on libpnet sample: https://github.com/libpnet/libpnet/blob/master/examples/packetdump.rs
use std::{ io::BufReader};
use std::path::PathBuf;
use std::net::IpAddr;
use structopt::StructOpt;
use pcap::Capture;
use gamestreaming::pnet::util::MacAddr;
use gamestreaming::pnet::packet::ethernet::{EtherTypes, EthernetPacket};
use gamestreaming::pnet::packet::ipv4::Ipv4Packet;
use gamestreaming::pnet::packet::ipv6::Ipv6Packet;
use gamestreaming::pnet::packet::udp::UdpPacket;
use gamestreaming::pnet::packet::Packet;
use gamestreaming::webrtc::stun;
use gamestreaming::crypto;
use gamestreaming::webrtc::rtp;
use gamestreaming::teredo::{Teredo};

type Error = Box<dyn std::error::Error>;
type Result<T> = std::result::Result<T, Error>;

struct PcapParser {
    crypto_context: Option<crypto::MsSrtpCryptoContext>,
    xbox_mac: Option<MacAddr>
}

impl PcapParser{
    pub fn new(srtp_base64: Option<String>) -> Result<Self> {
        let mut crypto: Option<crypto::MsSrtpCryptoContext> = None;
        if let Some(key) = srtp_base64 {
            crypto = crypto::MsSrtpCryptoContext::from_base64(&key).ok()
        };

        Ok(Self { 
            crypto_context: crypto,
            xbox_mac: None
        })
    }

    fn handle_udp_packet(&mut self, source: (IpAddr, MacAddr), destination: (IpAddr, MacAddr), packet: &[u8]) {
        if let Some(udp) = UdpPacket::new(packet) {
            let payload = udp.payload();

            if let Some(teredo) = Ipv6Packet::new(payload) {
                if teredo.is_teredo()
                {
                    println!(
                        "TEREDO Packet {:?}", teredo);
                    if self.xbox_mac == None && udp.get_source() == 3074 {
                        self.xbox_mac.replace(source.1);
                    }
                }
            }
            else if stun::message::is_message(payload) {
                let mut stun_msg = stun::message::Message::new();
                stun_msg.raw = payload.to_vec();
                if let Ok(_) = stun_msg.decode()
                {
                    println!("STUN Packet: {}", stun_msg);
                } else {
                    println!("Malformed STUN packet");
                }
            }
            else {
                let mut reader = BufReader::new(payload);
                if let Ok(rtp_packet) = rtp::packet::Packet::unmarshal(&mut reader) {
                    if rtp_packet.header.version == 2 &&
                        rtp_packet.size() == payload.len() &&
                        rtp_packet.header.marker == false {
                        println!("RTP Packet: {}", rtp_packet);    
                    }
                }
            }
        }

        /*
        println!(
            "UDP Packet: {}:{} > {}:{}; length: {}",
            source.0,
            udp.get_source(),
            destination.0,
            udp.get_destination(),
            udp.get_length()
        );
        */
    }

    fn handle_ethernet_packet(&mut self, packet: &[u8]) {
        if let Some(ethernet) = EthernetPacket::new(&packet) {
            match ethernet.get_ethertype() {
                EtherTypes::Ipv4 => {
                    if let Some(header) = Ipv4Packet::new(ethernet.payload()) {
                        let source_addr = IpAddr::V4(header.get_source());
                        let dest_addr = IpAddr::V4(header.get_destination());
                        let protocol = header.get_next_level_protocol();
                        let payload = header.payload();

                        self.handle_udp_packet(
                            (source_addr, ethernet.get_source()), 
                            (dest_addr, ethernet.get_destination()),
                            payload
                        );
                    } else {
                        println!("Malformed IPv4 Packet");
                    }
                },
                EtherTypes::Ipv6 => {
                    if let Some(header) = Ipv6Packet::new(ethernet.payload()) {
                        let source_addr = IpAddr::V6(header.get_source());
                        let dest_addr = IpAddr::V6(header.get_destination());
                        let protocol = header.get_next_header();
                        let payload = header.payload();

                        self.handle_udp_packet(
                            (source_addr, ethernet.get_source()), 
                            (dest_addr, ethernet.get_destination()),
                            payload
                        );
                    } else {
                        println!("Malformed IPv6 Packet");
                    }
                },
                _ => println!(
                    "Unhandled packet: {} > {}; ethertype: {:?} length: {}",
                    ethernet.get_source(),
                    ethernet.get_destination(),
                    ethernet.get_ethertype(),
                    ethernet.packet().len()
                ),
            }
        } else {
            println!("Failed to convert raw data to EthernetPacket");
        }
    }
}

#[derive(Debug, StructOpt)]
#[structopt(name = "XCloud pcap parser", about = "Parses pcap/-ng files for analysis.")]
struct Opt {
    /// Enable debug output
    #[structopt(short, long)]
    debug: bool,

    /// Input file
    #[structopt(parse(from_os_str))]
    input_file: PathBuf,

    /// SRTP Master bytes
    #[structopt(short, long)]
    srtp_key: Option<String>
}

fn main() {
    let opt = Opt::from_args();

    println!("Using SRTP key: {:?}", opt.srtp_key);
    
    let mut cap = Capture::from_file(opt.input_file)
        .expect("Failed to open input file");

    let mut parser = PcapParser::new(opt.srtp_key)
        .expect("Failed to create parser");

    while let Ok(packet) = cap.next() {
        parser.handle_ethernet_packet(&packet.data);
    }
}