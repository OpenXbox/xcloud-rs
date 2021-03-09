use std::convert::TryInto;

/// Implementation of MS-SRTP
/// Source: https://docs.microsoft.com/en-us/openspecs/office_protocols/ms-srtp/bf622cc1-9fb5-4fa2-b18d-239a84dcca65
///
/// SRTP requires that each endpoint in an SRTP session maintain cryptographic contexts. For more information, see
/// [RFC3711] section 3.2.3. This protocol maintains cryptographic contexts differently from SRTP [RFC3711].
/// 
/// This protocol maintains two cryptographic contexts per SRTP session:
/// 
/// - One for all media streams on the send direction.
/// - One for all media streams on the receive direction.
/// 
/// This protocol supports multiple media streams sharing the same SRTP session. Each media stream MUST be uniquely
/// identified by one Synchronization Source (SSRC). This protocol maintains per SSRC transform independent
/// parameters in cryptographic contexts, as specified in section 3.1.3.2.
/// 
/// When sending or receiving an SRTP packet, this protocol first uses the SRTP session and direction to identify
/// the cryptographic context, then uses the SSRC in the packet to decide the per SSRC transform independent
/// parameters in the cryptographic context.

use crate::webrtc::srtp::{protection_profile, context};
use crate::webrtc::rtp::header::Header;

type Error = Box<dyn std::error::Error>;
type Result<T> = std::result::Result<T, Error>;

pub struct MsSrtpCryptoContext {
    crypto_ctx_in: context::Context,
    crypto_ctx_out: context::Context,
}

impl MsSrtpCryptoContext {
    pub fn new(master_key: [u8; 16], master_salt: [u8; 14]) -> Result<Self> {
        Ok(Self {
            crypto_ctx_in: context::Context::new(
                &master_key,
                &master_salt,
                protection_profile::ProtectionProfile::AEADAES128GCM_MS_SRTP,
                None,
                None,
            )?,
            crypto_ctx_out: context::Context::new(
                &master_key,
                &master_salt,
                protection_profile::ProtectionProfile::AEADAES128GCM_MS_SRTP,
                None,
                None,
            )?,
        })
    }

    pub fn from_base64(master_bytes: &str) -> Result<Self> {
        let master_bytes = base64::decode(master_bytes)?;
        Self::new(
            master_bytes[..16].try_into()?,
            master_bytes[16..].try_into()?
        )
    }

    pub fn decrypt_rtp_with_header(
        &mut self,
        encrypted: &[u8],
        header: &Header
    ) -> Result<Vec<u8>> {
        Ok(self.crypto_ctx_out.decrypt_rtp_with_header(encrypted, header)?)
    }

    pub fn decrypt_rtp(&mut self, encrypted: &[u8]) -> Result<Vec<u8>> {
        Ok(self.crypto_ctx_in.decrypt_rtp(encrypted)?)
    }

    pub fn encrypt_rtp_with_header(
        &mut self,
        plaintext: &[u8],
        header: &Header
    ) -> Result<Vec<u8>> {
        Ok(self.crypto_ctx_out.encrypt_rtp_with_header(plaintext, header)?)
    }

    pub fn encrypt_rtp(&mut self, plaintext: &[u8]) -> Result<Vec<u8>> {
        Ok(self.crypto_ctx_out.encrypt_rtp(plaintext)?)
    }

    pub fn decrypt_rtp_as_host(&mut self, encrypted: &[u8]) -> Result<Vec<u8>> {
        Ok(self.crypto_ctx_out.decrypt_rtp(encrypted)?)
    }

    pub fn encrypt_rtp_as_host(&mut self, encrypted: &[u8]) -> Result<Vec<u8>> {
        Ok(self.crypto_ctx_in.decrypt_rtp(encrypted)?)
    }
}

#[cfg(test)]
mod test {
    use super::MsSrtpCryptoContext;

    pub const SRTP_KEY: &str = "RdHzuLLVGuO1aHILIEVJ1UzR7RWVioepmpy+9SRf";

    #[test]
    fn test_decrypt() {
        let data = include_bytes!("../testdata/rtp_connection_probing.bin");
        let mut context = MsSrtpCryptoContext::from_base64(SRTP_KEY)
            .expect("Failed to initialize crypto context");

        assert_eq!(data.len(), 1364);

        let decrypted = context.decrypt_rtp(data)
            .expect("Failed to decrypt packet");
        
        assert_eq!(decrypted.len(), 1348);
    }
}