use byteorder::*;
use std::io::{Read, Seek};

use crate::{crypto::OneShotHasher, packets::serializing::Deserialize};

use hmac::Hmac;
use sha2::Sha256;

type Error = Box<dyn std::error::Error>;
type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PingPayload {
    pub ping_type: u8,
    pub flags: u8,
    pub sequence_num: u32,
    pub signature: Vec<u8>,
}

impl PingPayload {
    fn new_request(sequence: u32, signing_context: &mut Hmac<Sha256>) -> Self {
        Self {
            ping_type: 0x01,
            flags: 0x00,
            sequence_num: sequence,
            signature: signing_context
                .hash_oneshot(&sequence.to_le_bytes())
                .expect("Failed to sign"),
        }
    }

    fn new_ack(sequence: u32, signing_context: &mut Hmac<Sha256>) -> Self {
        Self {
            ping_type: 0x01,
            flags: 0xFF,
            sequence_num: sequence,
            signature: signing_context
                .hash_oneshot(&sequence.to_le_bytes())
                .expect("Failed to sign"),
        }
    }

    /*
    fn is_signature_valid(&self, mut signing_context: Hmac<Sha256>) -> Result<()> {
        signing_context.update(&self.sequence_num.to_le_bytes());

        let result = signing_context.verify(&self.signature)
            .expect("Signature verification failed");

        Ok(result)
    }
     */
}

impl Deserialize for PingPayload {
    fn deserialize<T: Read + Seek>(reader: &mut T) -> Result<Self> {
        let mut signature = vec![0; 0x20];

        let ping_type = reader.read_u8()?;
        let flags = reader.read_u8()?;
        let sequence_num = reader.read_u32::<LittleEndian>()?;
        reader.read_exact(&mut signature)?;

        Ok(Self {
            ping_type,
            flags,
            sequence_num,
            signature,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PingPacket {
    Request(PingPayload),
    Response(PingPayload),
}

impl Deserialize for PingPacket {
    fn deserialize<T: Read + Seek>(reader: &mut T) -> Result<Self> {
        let body = PingPayload::deserialize(reader)?;

        match body.flags {
            0x00 => Ok(PingPacket::Request(body)),
            0xFF => Ok(PingPacket::Response(body)),
            _ => Err(format!("PingBody with unhandled flags: {:?}", body.flags))?,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::crypto::MsSrtpCryptoContext;
    use hex;
    use std::io::Cursor;

    #[test]
    fn deserialize_ping_packet() {
        let packet_data = hex::decode(
            "ffff010000000000d0c87bfa07d4e7fc9909d96e3cb3977d5232bbb391932236d56411f82d103bd5",
        )
        .expect("Failed to hex-decode ping packet");

        let ctx = MsSrtpCryptoContext::from_base64("19J859/D70mZNfu9tEUdxgUVVMbRDkV/L2LavviX")
            .expect("Failed to create MS-SRTP context");

        // First two bytes of the udp payload is salt / connection id
        let _ping_signing_ctx = ctx
            .get_ping_signing_ctx(&packet_data[..2])
            .expect("Failed to get ping signing context");

        // Udp payload + 2 is ping packet/payload
        let mut reader = Cursor::new(&packet_data[2..]);
        let packet =
            PingPacket::deserialize(&mut reader).expect("Failed to deserialize Ping packet");

        match packet {
            PingPacket::Request(request) => {
                assert_eq!(request.ping_type, 0x01);
                assert_eq!(request.flags, 0x00);
                assert_eq!(request.sequence_num, 0x0);
                assert_eq!(
                    &hex::encode(&request.signature),
                    "d0c87bfa07d4e7fc9909d96e3cb3977d5232bbb391932236d56411f82d103bd5"
                );

                /*
                request.is_signature_valid(ping_signing_ctx.clone())
                    .expect("Test Signature verification failed");
                 */
            }
            _ => {
                panic!("Deserialized into invalid ping packet")
            }
        }
    }

    #[test]
    fn init_ping_body() {
        let ctx = MsSrtpCryptoContext::from_base64("19J859/D70mZNfu9tEUdxgUVVMbRDkV/L2LavviX")
            .expect("Failed to create MS-SRTP context");

        let salt = &hex::decode("ffff").expect("Failed to hex-decode salt");

        let mut ping_signing_ctx = ctx
            .get_ping_signing_ctx(salt)
            .expect("Failed to get ping signing context");

        let body = PingPayload::new_request(0, &mut ping_signing_ctx);

        assert_eq!(
            hex::encode(&body.signature),
            "d0c87bfa07d4e7fc9909d96e3cb3977d5232bbb391932236d56411f82d103bd5"
        );
    }
}
