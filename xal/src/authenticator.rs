use super::{app_params, models, models::request, models::response, request_signer};
use base64;
use correlation_vector::vector_impl;
use rand;
use rand_core::RngCore;
use reqwest;
use serde::Serialize;
use sha2::{Digest, Sha256};
use uuid;

type Error = Box<dyn std::error::Error>;
type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
struct XalAuthenticator {
    client_id: uuid::Uuid,
    client_params: models::XalClientParameters,
    ms_cv: vector_impl::CorrelationVector,
    client: reqwest::Client,
    request_signer: request_signer::RequestSigner,
}

impl Default for XalAuthenticator {
    fn default() -> Self {
        Self {
            client_id: uuid::Uuid::new_v4(),
            client_params: app_params::get_android_xboxbeta_params(),
            ms_cv: vector_impl::CorrelationVector::default(),
            client: reqwest::Client::new(),
            request_signer: request_signer::RequestSigner {
                signing_key_pem: "".to_owned(),
                signing_policy: models::SigningPolicy::default(),
            },
        }
    }
}

impl XalAuthenticator {
    fn get_random_bytes() -> Vec<u8> {
        let mut result = [0u8; 16];
        rand::thread_rng().fill_bytes(&mut result);

        result.to_vec()
    }

    pub fn generate_code_verifier(bytes: Option<Vec<u8>>) -> Result<String> {
        // https://tools.ietf.org/html/rfc7636
        let random_bytes = bytes.unwrap_or(XalAuthenticator::get_random_bytes());

        // Base64 urlsafe encoding WITH stripping trailing '='
        let code_verifier = base64::encode_config(random_bytes, base64::URL_SAFE_NO_PAD);
        let code_verifier = code_verifier.trim_end_matches('=');

        assert_eq!(code_verifier.len() >= 43, true);
        assert_eq!(code_verifier.len() <= 128, true);

        Ok(code_verifier.to_owned())
    }

    pub fn get_code_challenge_from_code_verifier(code_verifier: String) -> Result<String> {
        let code_challenge = Sha256::digest(code_verifier.as_bytes());

        // Base64 urlsafe encoding WITH stripping trailing '='
        let code_challenge = base64::encode_config(code_challenge, base64::URL_SAFE_NO_PAD);
        let code_challenge = code_challenge.trim_end_matches('=');

        Ok(code_challenge.to_owned())
    }

    pub fn generate_random_state() -> String {
        let state = uuid::Uuid::new_v4().to_hyphenated().to_string();

        base64::encode(state)
    }
}

impl XalAuthenticator {
    async fn call_oauth20_token_endpoint<T>(&mut self, json_body: T) -> Result<reqwest::Response>
    where
        T: Serialize,
    {
        let resp = self
            .client
            .post("https://login.live.com/oauth20_token.srf")
            .header("MS-CV", "")
            .json(&json_body)
            .send()
            .await?;

        Ok(resp)
    }

    pub async fn exchange_code_for_token(
        &mut self,
        authorization_code: &str,
        code_verifier: &str,
    ) -> Result<response::WindowsLiveTokenResponse> {
        let json_body = request::WindowsLiveTokenRequest {
            client_id: self.client_params.app_id,
            code: Some(authorization_code),
            code_verifier: Some(code_verifier),
            grant_type: "authorization_code",
            redirect_uri: Some(self.client_params.redirect_uri),
            scope: "service::user.auth.xboxlive.com::MBI_SSL",
            refresh_token: None,
        };

        let resp = self.call_oauth20_token_endpoint(json_body).await?;
        Ok(serde_json::from_str(&resp.text().await?)?)
    }

    pub async fn exchange_refresh_token_for_xcloud_transfer_token(
        &mut self,
        refresh_token: &str,
    ) -> Result<response::XCloudTokenResponse> {
        let json_body = request::WindowsLiveTokenRequest {
            client_id: self.client_params.app_id,
            grant_type: "refresh_token",
            scope:
                "service::http://Passport.NET/purpose::PURPOSE_XBOX_CLOUD_CONSOLE_TRANSFER_TOKEN",
            refresh_token: Some(refresh_token),
            code: None,
            code_verifier: None,
            redirect_uri: None,
        };

        let resp = self.call_oauth20_token_endpoint(json_body).await?;
        Ok(serde_json::from_str(&resp.text().await?)?)
    }

    pub async fn refresh_token(
        &mut self,
        refresh_token: &str,
    ) -> Result<response::WindowsLiveTokenResponse> {
        let json_body = request::WindowsLiveTokenRequest {
            client_id: self.client_params.app_id,
            grant_type: "refresh_token",
            scope: "service::user.auth.xboxlive.com::MBI_SSL",
            refresh_token: Some(refresh_token),
            redirect_uri: Some(self.client_params.redirect_uri),
            code: None,
            code_verifier: None,
        };

        let resp = self.call_oauth20_token_endpoint(json_body).await?;
        Ok(serde_json::from_str(&resp.text().await?)?)
    }
}

impl XalAuthenticator {
    pub async fn get_endpoints(&self) -> Result<response::TitleEndpointsResponse> {
        let resp = self
            .client
            .get("https://title.mgt.xboxlive.com/titles/default/endpoints")
            .header("x-xbl-contract-version", "1")
            .query(&[("type", 1)])
            .send()
            .await?
            .json::<response::TitleEndpointsResponse>()
            .await?;

        Ok(resp)
    }

    pub async fn get_device_token(&mut self) -> Result<response::XADResponse> {
        let client_uuid: String = match self.client_params.device_type {
            // {decf45e4-945d-4379-b708-d4ee92c12d99}
            models::DeviceType::ANDROID => [
                "{".to_string(),
                self.client_id.to_hyphenated().to_string(),
                "}".to_string(),
            ]
            .concat(),

            // DECF45E4-945D-4379-B708-D4EE92C12D99
            models::DeviceType::IOS => self.client_id.to_hyphenated().to_string().to_uppercase(),
            // Unknown
            _ => self.client_id.to_hyphenated().to_string(),
        };

        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("x-xbl-contract-version", "1".parse()?);
        headers.insert("MS-CV", self.ms_cv.increment().parse()?);

        let json_body = request::XADRequest {
            relying_party: "http://auth.xboxlive.com",
            token_type: "JWT",
            properties: request::XADProperties {
                auth_method: "ProofOfPossession",
                id: client_uuid.as_str(),
                device_type: self.client_params.device_type.into(),
                version: self.client_params.client_version,
                proof_key: self.request_signer.get_proof_key(),
            },
        };

        let mut request = self
            .client
            .post("https://device.auth.xboxlive.com/device/authenticate")
            .headers(headers)
            .json(&json_body)
            .build()?;
        self.request_signer.sign_request(&mut request, None)?;
        let resp = self.client.execute(request).await?;
        Ok(serde_json::from_str(&resp.text().await?)?)
    }
    pub async fn do_sisu_authentication(
        &mut self,
        device_token: &str,
        code_challenge: &str,
        state: &str,
    ) -> Result<response::SisuAuthenticationResponse> {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("x-xbl-contract-version", "1".parse()?);
        headers.insert("MS-CV", self.ms_cv.increment().parse()?);

        let json_body = request::SisuAuthenticationRequest {
            app_id: self.client_params.app_id,
            title_id: self.client_params.title_id,
            redirect_uri: self.client_params.redirect_uri,
            device_token: device_token,
            sandbox: "RETAIL",
            token_type: "code",
            offers: vec!["service::user.auth.xboxlive.com::MBI_SSL"],
            query: request::SisuQuery {
                display: self.client_params.query_display,
                code_challenge: code_challenge,
                code_challenge_method: "S256",
                state: state,
            },
        };

        let mut request = self
            .client
            .post("https://sisu.xboxlive.com/authenticate")
            .headers(headers)
            .json(&json_body)
            .build()?;

        self.request_signer.sign_request(&mut request, None)?;
        let resp = self.client.execute(request).await?;
        Ok(serde_json::from_str(&resp.text().await?)?)
    }
    pub async fn do_sisu_authorization(
        &mut self,
        sisu_session_id: &str,
        access_token: &str,
        device_token: &str,
    ) -> Result<response::SisuAuthorizationResponse> {
        let json_body = request::SisuAuthorizationRequest {
            access_token: &format!("t={}", access_token),
            app_id: self.client_params.app_id,
            device_token: device_token,
            sandbox: "RETAIL",
            site_name: "user.auth.xboxlive.com",
            session_id: sisu_session_id,
            proof_key: self.request_signer.get_proof_key(),
        };

        let mut request = self
            .client
            .post("https://sisu.xboxlive.com/authorize")
            .header("MS-CV", self.ms_cv.increment())
            .json(&json_body)
            .build()?;

        self.request_signer.sign_request(&mut request, None)?;
        let resp = self.client.execute(request).await?;
        Ok(serde_json::from_str(&resp.text().await?)?)
    }
    pub async fn do_xsts_authorization(
        &mut self,
        device_token: &str,
        title_token: &str,
        user_token: &str,
        relying_party: &str,
    ) -> Result<response::XSTSResponse> {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("x-xbl-contract-version", "1".parse()?);
        headers.insert("MS-CV", self.ms_cv.increment().parse()?);

        let json_body = request::XSTSRequest {
            relying_party: relying_party,
            token_type: "JWT",
            properties: request::XSTSProperties {
                sandbox_id: "RETAIL",
                device_token: device_token,
                title_token: title_token,
                user_tokens: vec![user_token],
            },
        };

        let mut request = self
            .client
            .post("https://xsts.auth.xboxlive.com/xsts/authorize")
            .headers(headers)
            .json(&json_body)
            .build()?;

        self.request_signer.sign_request(&mut request, None)?;
        let resp = self.client.execute(request).await?;
        Ok(serde_json::from_str(&resp.text().await?)?)
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test() {
        assert_eq!(true, true);
    }
}
