use crate::crypto::OneShotHasher;
use deku::prelude::*;
use hmac::Hmac;
use sha2::Sha256;

#[derive(Debug, Clone, DekuRead, DekuWrite, PartialEq, Eq)]
#[deku(type = "u8")]
pub enum PingFlag {
    Request = 0x00,
    Response = 0xFF,
}

#[derive(Debug, Clone, DekuRead, DekuWrite, PartialEq, Eq)]
pub struct PingPayload {
    pub ping_type: u8,
    pub flags: PingFlag,
    pub sequence_num: u32,
    #[deku(bytes_read = "32")]
    pub signature: Vec<u8>,
}

impl PingPayload {
    fn new_request(sequence: u32, signing_context: &mut Hmac<Sha256>) -> Self {
        Self {
            ping_type: 0x01,
            flags: PingFlag::Request,
            sequence_num: sequence,
            signature: signing_context
                .hash_oneshot(&sequence.to_le_bytes())
                .expect("Failed to sign"),
        }
    }

    fn new_ack(sequence: u32, signing_context: &mut Hmac<Sha256>) -> Self {
        Self {
            ping_type: 0x01,
            flags: PingFlag::Response,
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

#[cfg(test)]
mod test {
    use super::*;
    use crate::crypto::MsSrtpCryptoContext;
    use hex;

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
        let (rest, packet) = PingPayload::from_bytes((&packet_data[2..], 0))
            .expect("Failed to deserialize Ping packet");

        assert_eq!(packet.ping_type, 0x01);
        assert_eq!(packet.flags, PingFlag::Request);
        assert_eq!(packet.sequence_num, 0x0);
        assert_eq!(
            &hex::encode(&packet.signature),
            "d0c87bfa07d4e7fc9909d96e3cb3977d5232bbb391932236d56411f82d103bd5"
        );
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
