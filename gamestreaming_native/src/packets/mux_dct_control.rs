use deku::prelude::*;

/*
RTP: MuxDCTControl Seq: 5, ts: 0, ssrc: 1024
|14c10af4 01640064 00020000 002e004d| .....d.d.......M 00000000
|6963726f 736f6674 3a3a4261 7369783a| icrosoft::Basix: 00000010
|3a446374 3a3a4368 616e6e65 6c3a3a43| :Dct::Channel::C 00000020
|6c617373 3a3a436f 6e74726f 6c000000| lass::Control... 00000030
|00020000 00020000 00|                .........        00000040
                                                       00000049
RTP: MuxDCTControl Seq: 6, ts: 0, ssrc: 1024
|14c06400 65000300 00000200 00000200| ..d.e........... 00000000
|0000|                                ..               00000010
                                                       00000012



RTP: MuxDCTControl Seq: 8, ts: 0, ssrc: 1025
|45c06600 301d0000 002a6700 02000000| E.f.0....*g..... 00000000
|2a004d69 63726f73 6f66743a 3a426173| *.Microsoft::Bas 00000010
|69783a3a 4463743a 3a436861 6e6e656c| ix::Dct::Channel 00000020
|3a3a436c 6173733a 3a516f53 00000000| ::Class::QoS.... 00000030
|00000000 0000|                       ......           00000040
                                                       00000046
RTP: MuxDCTControl Seq: 9, ts: 0, ssrc: 1025
|04c06800 03000000 00000000 0000|     ..h...........   00000000



RTP: MuxDCTControl Seq: 10, ts: 0, ssrc: 1026
|04c06900 02000000 2c004d69 63726f73| ..i.....,.Micros 00000000
|6f66743a 3a426173 69783a3a 4463743a| oft::Basix::Dct: 00000010
|3a436861 6e6e656c 3a3a436c 6173733a| :Channel::Class: 00000020
|3a566964 656f0000 00000000 00000000| :Video.......... 00000030
                                                       00000040
RTP: MuxDCTControl Seq: 11, ts: 0, ssrc: 1026
|04c06a00 03000000 00000000 0000|     ..j...........   00000000
                                                       0000000e
*/

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlProtocolPacketType {
    Create = 2,
    Open = 3,
    Close = 4,
}

#[derive(Debug, Clone, DekuRead, DekuWrite, PartialEq, Eq)]
pub struct MuxDCTControlHeader {
    pub bla: u16,
    pub bla2: u16,
    pub woop: u16,
    pub woop2: u16,
}
