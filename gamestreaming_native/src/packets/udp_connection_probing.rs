use deku::prelude::*;

#[derive(Debug, Clone, DekuRead, DekuWrite, PartialEq, Eq)]
#[deku(type = "u16")]
pub enum ConnectionProbingType {
    Syn = 1,
    Ack = 2,
}

#[derive(Debug, Clone, DekuRead, DekuWrite, PartialEq, Eq)]
pub struct ConnectionProbingSyn {
    // TODO: Implement deku(until = "")
    // We likely have to pass the total packet size here as ctx
    // to calculate EOF.
    // See:
    //  <https://docs.rs/deku/latest/deku/attributes/#until>
    //  <https://docs.rs/deku/latest/deku/attributes/#ctx>
    #[deku(bytes_read = "5")]
    pub probe_data: Vec<u8>,
}

#[derive(Debug, Clone, DekuRead, DekuWrite, PartialEq, Eq)]
pub struct ConnectionProbingAck {
    pub accepted_packet_size: u16,
    pub appendix: u16,
}

#[derive(Debug, Clone, DekuRead, DekuWrite, PartialEq, Eq)]
pub struct ConnectionProbingPacket {
    pub packet_type: ConnectionProbingType,
    #[deku(cond = "*packet_type == ConnectionProbingType::Syn")]
    pub syn: Option<ConnectionProbingSyn>,
    #[deku(cond = "*packet_type == ConnectionProbingType::Ack")]
    pub ack: Option<ConnectionProbingAck>,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_deserialize_connection_probing_syn() {
        let buf: Vec<u8> = vec![1, 0, 2, 3, 4, 5, 6];

        let (rest, packet) =
            ConnectionProbingPacket::from_bytes((&buf, 0)).expect("Failed to parse packet");

        assert_eq!(rest.1, 0);

        let syn = packet.syn.expect("Syn portion not deserialized");
        assert_eq!(packet.packet_type, ConnectionProbingType::Syn);
        assert_eq!(syn.probe_data.len(), 5);
    }

    #[test]
    fn test_deserialize_connection_probing_ack() {
        let buf: Vec<u8> = vec![2, 0, 5, 0, 0, 0];

        let (rest, packet) =
            ConnectionProbingPacket::from_bytes((&buf, 0)).expect("Failed to parse packet");

        assert_eq!(rest.1, 0);

        let ack = packet.ack.expect("Ack portion not deserialized");
        assert_eq!(packet.packet_type, ConnectionProbingType::Ack);
        assert_eq!(ack.accepted_packet_size, 5);
        assert_eq!(ack.appendix, 0);
    }
}
