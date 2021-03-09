/// Based on libpnet sample: https://github.com/libpnet/libpnet/blob/master/examples/packetdump.rs
use std::{io::BufReader, convert::TryInto};
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
use gamestreaming::teredo::{Teredo, TeredoEndpoint};
use hexdump;

type Error = Box<dyn std::error::Error>;
type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
struct RtpPacketResult {
    is_client: bool,
    packet: Vec<u8>,
}

struct PcapParser {
    xbox_mac: Option<MacAddr>
}

impl PcapParser{
    pub fn new() -> Self {
        Self {
            xbox_mac: None
        }
    }

    fn handle_udp_packet(
        &mut self,
        source: (IpAddr, MacAddr),
        destination: (IpAddr, MacAddr),
        packet: &[u8],
        teredo_wrapped: bool
    ) -> Result<Vec<u8>> {
        if let Some(udp) = UdpPacket::new(packet) {
            let payload = udp.payload();

            if let Some(teredo) = Ipv6Packet::new(payload) {
                if teredo.is_teredo()
                {
                    let teredo_src: TeredoEndpoint = teredo.get_source().try_into()?;
                    let teredo_dst: TeredoEndpoint = teredo.get_destination().try_into()?;

                    //println!("TEREDO Packet {:?}", teredo);
                    if self.xbox_mac == None && udp.get_source() == 3074 {
                        self.xbox_mac.replace(source.1);
                    }
                    return self.handle_udp_packet(
                        (IpAddr::V4(teredo_src.teredo_client_ipv4), source.1),
                        (IpAddr::V4(teredo_dst.teredo_client_ipv4), destination.1),
                        teredo.payload(),
                        true
                    );
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
                        rtp_packet.size() == payload.len() {

                        return Ok(payload.to_vec());
                    }
                }
                else {
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
            }
        }

        Err("Non-RTP packet")?
    }

    fn is_client_direction(&self, source_mac: MacAddr) -> bool {
        if let Some(xbox_mac) = self.xbox_mac {
            xbox_mac == source_mac
        }
        else {
            false
        }
    }

    fn handle_packet(&mut self, packet: &[u8]) -> Result<RtpPacketResult> {
        if let Some(ethernet) = EthernetPacket::new(&packet) {
            match ethernet.get_ethertype() {
                EtherTypes::Ipv4 => {
                    if let Some(header) = Ipv4Packet::new(ethernet.payload()) {
                        let source_addr = IpAddr::V4(header.get_source());
                        let source_mac = ethernet.get_source();
                        let dest_addr = IpAddr::V4(header.get_destination());
                        let dest_mac = ethernet.get_destination();
                        let protocol = header.get_next_level_protocol();
                        let payload = header.payload();

                        if let Ok(rtp_packet) = self.handle_udp_packet(
                            (source_addr, source_mac), 
                            (dest_addr, dest_mac),
                            payload,
                            false
                        ) {
                            return Ok(RtpPacketResult {
                                is_client: self.is_client_direction(source_mac),
                                packet: rtp_packet
                            });
                        }
                    } else {
                        println!("Malformed IPv4 Packet");
                    }
                },
                EtherTypes::Ipv6 => {
                    if let Some(header) = Ipv6Packet::new(ethernet.payload()) {
                        let source_addr = IpAddr::V6(header.get_source());
                        let source_mac = ethernet.get_source();
                        let dest_addr = IpAddr::V6(header.get_destination());
                        let dest_mac = ethernet.get_destination();
                        let protocol = header.get_next_header();
                        let payload = header.payload();

                        if let Ok(rtp_packet) = self.handle_udp_packet(
                            (source_addr, source_mac), 
                            (dest_addr, dest_mac),
                            payload,
                            false
                        ) {
                            return Ok(RtpPacketResult {
                                is_client: self.is_client_direction(source_mac),
                                packet: rtp_packet
                            });
                        }
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

        Err("Non-RTP packet")?
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

    let mut parser = PcapParser::new();

    let mut crypto_context: crypto::MsSrtpCryptoContext = {
        if let Some(key) = opt.srtp_key {
            crypto::MsSrtpCryptoContext::from_base64(&key)
                .expect("Failed to init crypto context")
        } else {
            let dummy_key = "RdHzuLLVGuO1aHILIEVJ1UzR7RWVioepmpy+9SRf";
            crypto::MsSrtpCryptoContext::from_base64(&dummy_key).ok()
                .expect("Failed to init dummy crypto context")
        }
    };

    while let Ok(packet) = cap.next() {
        if let Ok(rtp_response) = parser.handle_packet(&packet.data) {
            let packet = rtp_response.packet;

            let decryption_result = {
                if rtp_response.is_client {
                    println!("CLIENT -> XBOX");
                    crypto_context.decrypt_rtp(&packet)
                }
                else {
                    println!("XBOX -> CLIENT");
                    crypto_context.decrypt_rtp_as_host(&packet)
                }
            };

            if let Ok(plaintext) = decryption_result {
                let mut reader = BufReader::new(&plaintext[..]);
                if let Ok(rtp_packet) = rtp::packet::Packet::unmarshal(&mut reader) {
                    println!("RTP Header: {:?}", rtp_packet.header);
                    hexdump::hexdump(&rtp_packet.payload);
                }
            } else {
                println!("Failed to decrypt RTP");
            }
        }
    }
}