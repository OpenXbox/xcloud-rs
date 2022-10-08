use std::convert::{From, Into, TryFrom, TryInto};
use std::io;
use std::io::{Read, Write, Seek, SeekFrom, Cursor};
use byteorder::*;

use super::serializing::{Serialize, Deserialize};

type Error = Box<dyn std::error::Error>;
type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, PartialEq)]
pub struct ConnectionProbingSyn {
    pub msg_type: ConnectionProbingType,
    pub probe_data: Vec<u8>
}

impl Deserialize for ConnectionProbingSyn {
    fn deserialize<T: Read + Seek>(reader: &mut T) -> Result<Self>
    {
        let msg_type = reader.read_u16::<LittleEndian>()?.try_into()?;
        assert_eq!(msg_type, ConnectionProbingType::Syn);
        
        let mut probe_data = vec![];
        let _ = reader.read_to_end(&mut probe_data)?;

        Ok(Self {
            msg_type,
            probe_data
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConnectionProbingAck {
    pub msg_type: ConnectionProbingType,
    pub accepted_packet_size: u16,
    pub appendix: u16
}

impl Deserialize for ConnectionProbingAck {
    fn deserialize<T: Read + Seek>(reader: &mut T) -> Result<Self> {
        let msg_type = reader.read_u16::<LittleEndian>()?.try_into()?;
        assert_eq!(msg_type, ConnectionProbingType::Ack);

        let accepted_packet_size = reader.read_u16::<LittleEndian>()?;
        let appendix = reader.read_u16::<LittleEndian>()?;

        Ok(Self {
            msg_type,
            accepted_packet_size,
            appendix
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
#[repr(u16)]
pub enum ConnectionProbingType {
    Syn = 1,
    Ack = 2,
}

impl From<u16> for ConnectionProbingType {
    fn from(value: u16) -> Self {
        let z: ConnectionProbingType = unsafe { ::std::mem::transmute(value) };

        z
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionProbingPacket {
    Syn(ConnectionProbingSyn),
    Ack(ConnectionProbingAck)
}

impl Deserialize for ConnectionProbingPacket {
    fn deserialize<T: Read + Seek>(reader: &mut T) -> Result<Self> {
        // Read msg_type and then rewind the buffer
        let current_pos = reader.seek(SeekFrom::Current(0))?;
        let msg_type = reader.read_u16::<LittleEndian>()?.try_into()?;
        reader.seek(SeekFrom::Start(current_pos))?;

        let packet = match msg_type {
            ConnectionProbingType::Syn => {
                ConnectionProbingPacket::Syn(ConnectionProbingSyn::deserialize(reader)?)
            },
            ConnectionProbingType::Ack => {
                ConnectionProbingPacket::Ack(ConnectionProbingAck::deserialize(reader)?)
            }
        };

        Ok(packet)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_deserialize_connection_probing_syn() {
        let buf: Vec<u8> = vec![1, 0, 2, 3, 4, 5, 6];
        let mut reader = Cursor::new(&buf);

        let parsed = ConnectionProbingSyn::deserialize(&mut reader)
            .expect("Failed to deserialize");

        assert_eq!(parsed.msg_type, ConnectionProbingType::Syn);
        assert_eq!(parsed.probe_data.len(), 5);
    }

    #[test]
    fn test_deserialize_connection_probing_ack() {
        let buf: Vec<u8> = vec![2, 0, 5, 0, 0, 0];
        let mut reader = Cursor::new(&buf);

        let parsed = ConnectionProbingAck::deserialize(&mut reader)
            .expect("Failed to deserialize");

        assert_eq!(parsed.msg_type, ConnectionProbingType::Ack);
        assert_eq!(parsed.accepted_packet_size, 5);
        assert_eq!(parsed.appendix, 0);
    }

    #[test]
    fn test_deserialize_connection_probing_packet() {
        let buf: Vec<u8> = vec![2, 0, 5, 0, 0, 0];
        let mut reader = Cursor::new(&buf);

        let parsed = ConnectionProbingPacket::deserialize(&mut reader)
            .expect("Failed to deserialize");

        match parsed {
            ConnectionProbingPacket::Ack(packet) => {
                assert_eq!(packet.msg_type, ConnectionProbingType::Ack);
                assert_eq!(packet.accepted_packet_size, 5);
                assert_eq!(packet.appendix, 0);
            },
            _ => { panic!("Failed") }
        }
    }
}