use super::filetime::FileTime;
use super::{models, models::request};
use base64;
use chrono::prelude::*;
use ecdsa;
use josekit;
use reqwest;
use sha2::{Digest, Sha256};
use std::convert::TryInto;
use std::option::Option;
use url::Position;

type Error = Box<dyn std::error::Error>;
type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub struct XboxWebSignatureBytes {
    signing_policy_version: Vec<u8>,
    timestamp: Vec<u8>,
    signed_digest: Vec<u8>,
}

#[derive(Debug)]
pub struct XboxDigestToSign(Vec<u8>);

#[derive(Debug)]
pub struct XboxPlaintextMessageToHash(Vec<u8>);

impl XboxWebSignatureBytes {
    pub fn as_bytestream(&self) -> Vec<u8> {
        let mut bytes: Vec<u8> = Vec::new();
        bytes.extend_from_slice(self.signing_policy_version.as_slice());
        bytes.extend_from_slice(self.timestamp.as_slice());
        bytes.extend_from_slice(self.signed_digest.as_slice());

        bytes
    }

    pub fn as_base64(&self) -> String {
        base64::encode(self.as_bytestream())
    }

    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self {
            signing_policy_version: bytes[..4].to_vec(),
            timestamp: bytes[4..12].to_vec(),
            signed_digest: bytes[12..].to_vec(),
        }
    }

    pub fn from_base64(text: String) -> Self {
        let bytes = base64::decode(text).expect("Failed to deserialize base64 signature");
        XboxWebSignatureBytes::from_bytes(bytes)
    }
}

pub struct HttpRequestToSign<'a> {
    method: &'a str,
    path_and_query: &'a str,
    authorization: &'a str,
    body: &'a [u8],
}

#[derive(Debug)]
pub struct RequestSigner {
    pub signing_key_pem: String,
    pub signing_policy: models::SigningPolicy,
}

impl RequestSigner {
    pub fn generate_key() -> String {
        "<INVALID PEM>".to_owned()
    }

    pub fn get_proof_key(&self) -> request::ProofKey {
        request::ProofKey {
            usage: "sig",
            algorithm: "ES256",
            key_type: "EC",
            curve: "P-256",
            pubkey_x: "",
            pubkey_y: "",
        }
    }

    pub fn sign_request(
        &self,
        request: &mut reqwest::Request,
        timestamp: Option<DateTime<Utc>>,
    ) -> Result<()> {
        let url = request.url();

        let to_sign = HttpRequestToSign {
            method: &request.method().to_string().to_uppercase(),
            path_and_query: &url[Position::BeforePath..],
            authorization: request
                .headers()
                .get(reqwest::header::AUTHORIZATION)
                .unwrap()
                .to_str()?,
            body: request.body().unwrap().as_bytes().unwrap(),
        };

        let signature = self
            .sign(
                self.signing_policy.version,
                timestamp.unwrap_or(Utc::now()),
                to_sign,
            )
            .expect("Signing request failed!");

        request
            .headers_mut()
            .insert("Signature", signature.as_base64().parse()?);

        Ok(())
    }

    /// Sign
    pub fn sign(
        &self,
        signing_policy_version: i32,
        timestamp: DateTime<Utc>,
        request: HttpRequestToSign,
    ) -> Result<XboxWebSignatureBytes> {
        self.sign_raw(
            signing_policy_version,
            timestamp,
            request.method.to_owned(),
            request.path_and_query.to_owned(),
            request.authorization.to_owned(),
            request.body,
        )
    }

    fn sign_raw(
        &self,
        signing_policy_version: i32,
        timestamp: DateTime<Utc>,
        method: String,
        path_and_query: String,
        authorization: String,
        body: &[u8],
    ) -> Result<XboxWebSignatureBytes> {
        let signer = josekit::jws::ES256.signer_from_pem(&self.signing_key_pem)?;

        let filetime_bytes = timestamp.to_filetime().to_be_bytes();
        let signing_policy_version_bytes = signing_policy_version.to_be_bytes();

        let message = self
            .assemble_message_data(
                &signing_policy_version_bytes,
                &filetime_bytes,
                method,
                path_and_query,
                authorization,
                body,
                self.signing_policy.max_body_bytes,
            )
            .expect("Failed to assemble message data !");

        // Hash the data to be signed
        let digest = RequestSigner::digest(message)?;

        // Sign the digest (RFC6979)
        // FIXME: Sign the digest only.. need to find another lib that supports it...
        let signed_digest: Vec<u8> = signer.sign(&digest.0)?;

        // Return final signature
        Ok(XboxWebSignatureBytes {
            signing_policy_version: signing_policy_version_bytes.to_vec(),
            timestamp: filetime_bytes.to_vec(),
            signed_digest: signed_digest,
        })
    }

    fn assemble_message_data(
        &self,
        signing_policy_version: &[u8],
        timestamp: &[u8],
        method: String,
        path_and_query: String,
        authorization: String,
        body: &[u8],
        max_body_bytes: usize,
    ) -> Result<XboxPlaintextMessageToHash> {
        const NULL_BYTE: &[u8; 1] = &[0x00];

        let mut data = Vec::<u8>::new();
        // Signature version + null
        data.extend_from_slice(signing_policy_version);
        data.extend_from_slice(NULL_BYTE);

        // Timestamp + null
        data.extend_from_slice(timestamp);
        data.extend_from_slice(NULL_BYTE);

        // Method (uppercase) + null
        data.extend_from_slice(method.to_uppercase().as_bytes());
        data.extend_from_slice(NULL_BYTE);

        // Path and query + null
        data.extend_from_slice(path_and_query.as_bytes());
        data.extend_from_slice(NULL_BYTE);

        // Authorization (even if an empty string)
        data.extend_from_slice(authorization.as_bytes());
        data.extend_from_slice(NULL_BYTE);

        // Body
        let body_size_to_hash = std::cmp::min(max_body_bytes, body.len());
        data.extend_from_slice(&body[..body_size_to_hash]);
        data.extend_from_slice(NULL_BYTE);

        Ok(XboxPlaintextMessageToHash(data))
    }

    fn digest(message: XboxPlaintextMessageToHash) -> Result<XboxDigestToSign> {
        let digest = Sha256::digest(&message.0).to_vec();
        return Ok(XboxDigestToSign(digest));
    }
}

#[cfg(test)]
mod test {
    use super::{reqwest, FileTime, RequestSigner};
    use chrono::prelude::*;
    use hex_literal::hex;

    const PRIVATE_KEY_PEM: &'static str = "-----BEGIN EC PRIVATE KEY-----\n
    MHcCAQEEIObr5IVtB+DQcn25+R9n4K/EyUUSbVvxIJY7WhVeELUuoAoGCCqGSM49\n
    AwEHoUQDQgAEOKyCQ9qH5U4lZcS0c5/LxIyKvOpKe0l3x4Eg5OgDbzezKNLRgT28\n
    fd4Fq3rU/1OQKmx6jSq0vTB5Ao/48m0iGg==\n
    -----END EC PRIVATE KEY-----\n";

    fn get_request_signer() -> RequestSigner {
        RequestSigner {
            signing_key_pem: PRIVATE_KEY_PEM.to_owned(),
            signing_policy: Default::default(),
        }
    }

    #[test]
    fn sign() {
        let signer = get_request_signer();

        let dt = Utc.timestamp(1586999965, 0);

        let signature = signer
            .sign_raw(
                1,
                dt,
                "POST".to_owned(),
                "/path?query=1".to_owned(),
                "XBL3.0 x=userid;jsonwebtoken".to_owned(),
                "thebodygoeshere".as_bytes(),
            )
            .expect("Signing failed!");

        assert_eq!(signature.as_base64(), "AAAAAQHWE40Q98yAFe3R7GuZfvGA350cH7hWgg4HIHjaD9lGYiwxki6bNyGnB8dMEIfEmBiuNuGUfWjY5lL2h44X/VMGOkPIezVb7Q==");
    }

    #[test]
    fn data_to_hash() {
        let signer = get_request_signer();
        let signing_policy_version: i32 = 1;
        let ts_bytes = Utc.timestamp(1586999965, 0).to_filetime().to_be_bytes();

        let message_data = signer
            .assemble_message_data(
                &signing_policy_version.to_be_bytes(),
                &ts_bytes,
                "POST".to_owned(),
                "/path?query=1".to_owned(),
                "XBL3.0 x=userid;jsonwebtoken".to_owned(),
                "thebodygoeshere".as_bytes(),
                8192,
            )
            .expect("Failed to assemble message data");

        assert_eq!(
            message_data.0,
            hex!("000000010001d6138d10f7cc8000504f5354002f706174683f71756572793d310058424c332e3020783d7573657269643b6a736f6e776562746f6b656e00746865626f6479676f65736865726500").to_vec()
        );
    }

    #[test]
    fn hash() {
        let signer = get_request_signer();
        let signing_policy_version: i32 = 1;
        let ts_bytes = Utc.timestamp(1586999965, 0).to_filetime().to_be_bytes();

        let message_data = signer
            .assemble_message_data(
                &signing_policy_version.to_be_bytes(),
                &ts_bytes,
                "POST".to_owned(),
                "/path?query=1".to_owned(),
                "XBL3.0 x=userid;jsonwebtoken".to_owned(),
                "thebodygoeshere".as_bytes(),
                8192,
            )
            .unwrap();

        let digest = RequestSigner::digest(message_data).expect("Failed to hash message");

        assert_eq!(
            digest.0,
            hex!("f7d61b6f8d4dcd86da1aa8553f0ee7c15450811e7cd2759364e22f67d853ff50")
        );
    }

    #[test]
    fn sign_reqwest() {
        let signer = get_request_signer();
        let timestamp = Utc.timestamp(1586999965, 0);

        let client = reqwest::Client::new();
        let mut request = client
            .post("https://example.com/path")
            .query(&[("query", 1)])
            .header(
                reqwest::header::AUTHORIZATION,
                "XBL3.0 x=userid;jsonwebtoken",
            )
            .body("thebodygoeshere")
            .build()
            .unwrap();

        signer
            .sign_request(&mut request, Some(timestamp))
            .expect("FAILED signing request");

        let signature = request.headers().get("Signature");

        assert!(!signature.is_none());
        assert_eq!(
            signature.unwrap(),
            "AAAAAQHWE40Q98yAFe3R7GuZfvGA350cH7hWgg4HIHjaD9lGYiwxki6bNyGnB8dMEIfEmBiuNuGUfWjY5lL2h44X/VMGOkPIezVb7Q=="
        );
    }
}
