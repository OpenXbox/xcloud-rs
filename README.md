# XCloud / SmartGlass - New API in RUST

## Building

```text
git clone --recursive https://github.com/OpenXbox/xcloud-rs.git
cd xcloud-rs
cargo build
cargo test
```

## Usage

### PCAP Parser

To simply decrypt communication and print to terminal:

```sh
$ cargo run --bin pcap_parser -- --srtp-key <SRTP KEY BASE64> <PATH TO PCAP>
Using SRTP key: Some("<SRTP KEY>")
PCAP Decrypt path: None
STUN Packet: Binding request l=76 attrs=4 id=hvRAX7bbtFhz4GZP
STUN Packet: Binding request l=76 attrs=4 id=Q/ZCmawz/1kmzU2P
STUN Packet: Binding request l=76 attrs=4 id=nv5JP7xN12fgbyXu
STUN Packet: Binding request l=76 attrs=4 id=hvRAX7bbtFhz4GZP
STUN Packet: Binding request l=76 attrs=4 id=OuJFwaMs+G7QrfBu
STUN Packet: Binding success response l=48 attrs=3 id=OuJFwaMs+G7QrfBu
STUN Packet: Binding request l=80 attrs=5 id=cyFG476hFBbaeza+
STUN Packet: Binding success response l=48 attrs=3 id=cyFG476hFBbaeza+
ConnectionProbingPacket::Syn(DataLen=1434)
ConnectionProbingPacket::Syn(DataLen=1434)
ConnectionProbingPacket::Syn(DataLen=1418)
ConnectionProbingPacket::Syn(DataLen=1402)
ConnectionProbingPacket::Syn(DataLen=1386)
ConnectionProbingPacket::Syn(DataLen=1370)
ConnectionProbingPacket::Syn(DataLen=1354)
STUN Packet: Binding request l=76 attrs=4 id=nv5JP7xN12fgbyXu
STUN Packet: Binding request l=76 attrs=4 id=hvRAX7bbtFhz4GZP
STUN Packet: Binding success response l=48 attrs=3 id=hvRAX7bbtFhz4GZP
STUN Packet: Binding request l=76 attrs=4 id=BXVF8Y8k7Bex1R5Y
STUN Packet: Binding success response l=48 attrs=3 id=BXVF8Y8k7Bex1R5Y
ConnectionProbingPacket::Syn(DataLen=1334)
ConnectionProbingPacket::Syn(DataLen=1318)
ConnectionProbingPacket::Syn(DataLen=1302)
ConnectionProbingPacket::Syn(DataLen=1286)
ConnectionProbingPacket::Syn(DataLen=1270)
ConnectionProbingPacket::Syn(DataLen=1254)
ConnectionProbingPacket::Syn(DataLen=1434)
ConnectionProbingPacket::Ack(AcceptedSize=1434, Appendix=0)
ConnectionProbingPacket::Ack(AcceptedSize=1334, Appendix=0)
RTP: UDPKeepAlive Seq: 2, ts: 0, ssrc: 0
|00000000 09000000 64000000 00000000| ........d....... 00000000
|401f0000 00000000 0a000000 58020000| @...........X... 00000010
|88130000 00000000 0000|              ..........       00000020
...
RTP: MuxDCTControl Seq: 5, ts: 0, ssrc: 1024
|14c10af4 01640064 00020000 002e004d| .....d.d.......M 00000000
|6963726f 736f6674 3a3a4261 7369783a| icrosoft::Basix: 00000010
|3a446374 3a3a4368 616e6e65 6c3a3a43| :Dct::Channel::C 00000020
|6c617373 3a3a436f 6e74726f 6c000000| lass::Control... 00000030
|00020000 00020000 00|
```

To decrypt into new PCAP file

```sh
$ cargo run --bin pcap_parser -- --srtp-key <SRTP KEY BASE64> --decrypt-pcap <TARGET PLAINTEXT PCAP> <PATH TO PCAP>
Using SRTP key: Some("<SRTP KEY>")
PCAP Decrypt path: Some("plaintext.pcap")
STUN Packet: Binding request l=76 attrs=4 id=hvRAX7bbtFhz4GZP
STUN Packet: Binding request l=76 attrs=4 id=Q/ZCmawz/1kmzU2P
STUN Packet: Binding request l=76 attrs=4 id=nv5JP7xN12fgbyXu
STUN Packet: Binding request l=76 attrs=4 id=hvRAX7bbtFhz4GZP
STUN Packet: Binding request l=76 attrs=4 id=OuJFwaMs+G7QrfBu
STUN Packet: Binding success response l=48 attrs=3 id=OuJFwaMs+G7QrfBu
STUN Packet: Binding request l=80 attrs=5 id=cyFG476hFBbaeza+
STUN Packet: Binding success response l=48 attrs=3 id=cyFG476hFBbaeza+
STUN Packet: Binding request l=76 attrs=4 id=nv5JP7xN12fgbyXu
STUN Packet: Binding request l=76 attrs=4 id=hvRAX7bbtFhz4GZP
STUN Packet: Binding success response l=48 attrs=3 id=hvRAX7bbtFhz4GZP
STUN Packet: Binding request l=76 attrs=4 id=BXVF8Y8k7Bex1R5Y
STUN Packet: Binding success response l=48 attrs=3 id=BXVF8Y8k7Bex1R5Y
```
