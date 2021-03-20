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
use pbkdf2::pbkdf2;
use hmac::{digest, Hmac, Mac, NewMac};
use sha2::Sha256;

type Error = Box<dyn std::error::Error>;
type Result<T> = std::result::Result<T, Error>;

pub trait OneShotHasher {
    fn hash_oneshot(&mut self, data: &[u8]) -> Result<Vec<u8>>;
}

impl OneShotHasher for Hmac<Sha256> {
    fn hash_oneshot(&mut self, data: &[u8]) -> Result<Vec<u8>> {
        self.update(data);
        let signature = self.finalize_reset();

        Ok(signature.into_bytes()[..].to_vec())
    }
}

pub struct MsSrtpCryptoContext {
    crypto_ctx_in: context::Context,
    crypto_ctx_out: context::Context,
    master_key: Vec<u8>,
    master_salt: Vec<u8>
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
            master_key: master_key.to_vec(),
            master_salt: master_salt.to_vec()
        })
    }

    pub fn from_base64(master_bytes: &str) -> Result<Self> {
        let master_bytes = base64::decode(master_bytes)?;
        Self::new(
            master_bytes[..16].try_into()?,
            master_bytes[16..].try_into()?
        )
    }

    fn derive_hmac_key<T>(master_key: &[u8], salt: &[u8], iterations: u32, key_out: &mut [u8]) -> Result<()>
        where T: digest::Update + digest::BlockInput + digest::FixedOutput + digest::Reset + Default + Clone
    {
        pbkdf2::<Hmac<Sha256>>(master_key, salt, iterations, key_out);

        Ok(())
    }

    fn get_keyed_hasher<T>(hmac_key: &[u8]) -> Result<hmac::Hmac<T>>
    where T: digest::Update + digest::BlockInput + digest::FixedOutput + digest::Reset + Default + Clone
    {
        Ok(hmac::Hmac::<T>::new_varkey(hmac_key)?)
    }

    pub fn get_ping_signing_ctx(&self, salt: &[u8]) -> Result<Hmac<Sha256>>
    {
        if salt.len() != 2 {
            Err("Salt has invalid length, expected 2 bytes")?
        }

        let mut hmac_key: [u8; 0x20] = [0; 0x20];
        MsSrtpCryptoContext::derive_hmac_key::<Sha256>(
            &self.master_key,
            salt,
            100000,
            &mut hmac_key
        )?;

        Ok(MsSrtpCryptoContext::get_keyed_hasher::<Sha256>(&hmac_key)?)
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
    use hex;
    use hmac::Mac;
    use super::*;

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

    #[test]
    fn test_ping_key_derivation() {

        let mut hmac_key: [u8; 0x20] = [0; 0x20];
        MsSrtpCryptoContext::derive_hmac_key::<Sha256>(
            &hex::decode("d7d27ce7dfc3ef499935fbbdb4451dc6").unwrap(),
            &hex::decode("ffff").unwrap(),
            100000,
            &mut hmac_key
        ).expect("Failed to derive hmac key");
        
        assert_eq!(&hex::encode(hmac_key), "9dda3a76d9e73b41ad8b37881e9d5af973271573d2fd3783dd6650b9840afb94");
    }

    #[test]
    fn test_keyed_hasher() {
        let hmac_key = hex::decode("9dda3a76d9e73b41ad8b37881e9d5af973271573d2fd3783dd6650b9840afb94")
            .expect("Failed to hexdecode hmac key");
        let mut hasher = MsSrtpCryptoContext::get_keyed_hasher::<Sha256>(&hmac_key)
            .expect("Failed to derive HMAC");
        
        hasher.update(&hex::decode("00000000").unwrap());
        let signature = &hasher.finalize().into_bytes()[..];

        assert_eq!(&hex::encode(signature), "d0c87bfa07d4e7fc9909d96e3cb3977d5232bbb391932236d56411f82d103bd5");
    }

    #[test]
    fn test_get_ping_key_context() {
        let ctx = MsSrtpCryptoContext::from_base64("19J859/D70mZNfu9tEUdxgUVVMbRDkV/L2LavviX")
            .expect("Failed to create MS-SRTP context");
        
        let mut ping_signing_ctx = ctx.get_ping_signing_ctx(&hex::decode("ffff").unwrap())
            .expect("Failed to create ping signing context");

        ping_signing_ctx.update(&hex::decode("00000000").unwrap());
        let signature = &ping_signing_ctx.finalize().into_bytes()[..];

        assert_eq!(&hex::encode(signature), "d0c87bfa07d4e7fc9909d96e3cb3977d5232bbb391932236d56411f82d103bd5");
    }

}