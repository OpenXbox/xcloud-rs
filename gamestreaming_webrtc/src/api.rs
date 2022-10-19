use reqwest::{header, header::HeaderMap, Client, ClientBuilder, StatusCode, Url};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json;
use thiserror::Error;
use webrtc::ice_transport::ice_candidate::RTCIceCandidateInit;

#[derive(Error, Debug)]
pub enum GssvApiError {
    #[error(transparent)]
    HttpError(#[from] reqwest::Error),
    #[error(transparent)]
    Serialization(#[from] serde_json::error::Error),
    #[error("Unknown error")]
    Unknown,
}

/// Gamestreaming API Client
pub struct GssvApi {
    client: Client,
    base_url: Url,
    pub platform: &'static str,
}

impl GssvApi {
    fn new(base_url: Url, gssv_token: &str, platform: &'static str) -> Self {
        let mut headers = header::HeaderMap::new();

        let mut auth_value = header::HeaderValue::from_str(&format!("Bearer {}", gssv_token))
            .expect("Failed assembling auth header");
        auth_value.set_sensitive(true);
        headers.insert(header::AUTHORIZATION, auth_value);

        Self {
            client: ClientBuilder::new()
                .default_headers(headers)
                .build()
                .expect("Failed to build client"),
            base_url,
            platform,
        }
    }

    async fn login(offering_id: &str, token: &str) -> Result<LoginResponse, GssvApiError> {
        let login_url = format!(
            "https://{}.gssv-play-prod.xboxlive.com/v2/login/user",
            offering_id
        );
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-gssv-client",
            "XboxComBrowser"
                .parse()
                .map_err(|_| GssvApiError::Unknown)?,
        );

        let client = reqwest::Client::new();
        client
            .post(login_url)
            .headers(headers)
            .json(&LoginRequest {
                token: token.into(),
                offering_id: offering_id.into(),
            })
            .send()
            .await
            .map_err(GssvApiError::HttpError)?
            .json::<LoginResponse>()
            .await
            .map_err(GssvApiError::HttpError)
    }

    pub async fn login_xhome(token: &str) -> Result<Self, GssvApiError> {
        let resp = GssvApi::login("xhome", token).await?;

        Ok(Self::new(
            Url::parse(&resp.offering_settings.regions.first().unwrap().base_uri).unwrap(),
            &resp.gs_token,
            "home",
        ))
    }

    pub async fn login_xcloud(token: &str) -> Result<Self, GssvApiError> {
        let resp = GssvApi::login("xgpuweb", token).await?;

        Ok(Self::new(
            Url::parse(&resp.offering_settings.regions.first().unwrap().base_uri).unwrap(),
            &resp.gs_token,
            "cloud",
        ))
    }

    fn url(&self, path: &str) -> Url {
        self.base_url.join(path).unwrap()
    }

    fn session_url(&self, session: &SessionResponse, path: &str) -> Url {
        self.base_url
            .join(&session.session_path)
            .unwrap()
            .join(path)
            .unwrap()
    }

    async fn get_json<T>(&self, url: Url, headers: Option<HeaderMap>) -> Result<T, GssvApiError>
    where
        T: DeserializeOwned,
    {
        let mut req = self.client.get(url);

        if let Some(headers) = headers {
            req = req.headers(headers);
        }

        req.send()
            .await
            .map_err(GssvApiError::HttpError)?
            .json::<T>()
            .await
            .map_err(GssvApiError::HttpError)
    }

    async fn post_json<RQ, RS>(
        &self,
        url: Url,
        request_body: RQ,
        headers: Option<HeaderMap>,
    ) -> Result<RS, GssvApiError>
    where
        RQ: Serialize,
        RS: DeserializeOwned,
    {
        let mut req = self.client.post(url);

        if let Some(headers) = headers {
            req = req.headers(headers);
        }

        req.json(&request_body)
            .send()
            .await
            .map_err(GssvApiError::HttpError)?
            .json::<RS>()
            .await
            .map_err(GssvApiError::HttpError)
    }

    pub async fn get_consoles(&self) -> Result<ConsolesResponse, GssvApiError> {
        self.get_json(self.url("/v6/servers/home"), None).await
    }

    pub async fn get_titles(&self) -> Result<TitlesResponse, GssvApiError> {
        self.get_json(self.url("/v1/titles"), None).await
    }

    pub async fn start_session(
        &self,
        server_id: Option<&str>,
        title_id: Option<&str>,
    ) -> Result<SessionResponse, GssvApiError> {
        let device_info = DeviceInfo {
            app_info: AppInfo {
                env: AppEnvironment {
                    client_app_id: "Microsoft.GamingApp".into(),
                    client_app_type: "native".into(),
                    client_app_version: "2203.1001.4.0".into(),
                    client_sdk_version: "5.3.0".into(),
                    http_environment: "prod".into(),
                    sdk_install_id: "".into(),
                },
            },
            dev: DevInfo {
                hw: DevHardwareInfo {
                    make: "Micro-Star International Co., Ltd.".into(),
                    model: "GS66 Stealth 10SGS".into(),
                    sdk_type: "native".into(),
                },
                os: DevOsInfo {
                    name: "Windows 10 Pro".into(),
                    ver: "19041.1.amd64fre.vb_release.191206-1406".into(),
                },
                display_info: DevDisplayInfo {
                    dimensions: DevDisplayDimensions {
                        width_in_pixels: 1920,
                        height_in_pixels: 1080,
                    },
                    pixel_density: DevDisplayPixelDensity { dpi_x: 1, dpi_y: 1 },
                },
            },
        };

        let devinfo_str =
            serde_json::to_string(&device_info).map_err(GssvApiError::Serialization)?;

        let mut headers = HeaderMap::new();
        headers.insert(
            "X-MS-Device-Info",
            devinfo_str.parse().map_err(|_| GssvApiError::Unknown)?,
        );
        headers.insert(
            "User-Agent",
            devinfo_str.parse().map_err(|_| GssvApiError::Unknown)?,
        );

        let request_body = GssvSessionConfig {
            title_id: title_id.unwrap_or("").into(),
            system_update_group: "".into(),
            server_id: server_id.unwrap_or("").into(),
            fallback_region_names: vec![],
            settings: GssvSessionSettings {
                nano_version: "V3;WebrtcTransport.dll".into(),
                enable_text_to_speech: false,
                high_contrast: 0,
                locale: "en-US".into(),
                use_ice_connection: false,
                timezone_offset_minutes: 120,
                sdk_type: "web".into(),
                os_name: "windows".into(),
            },
        };

        self.post_json(
            self.url(&format!("/v5/sessions/{}/play", self.platform)),
            &request_body,
            Some(headers),
        )
        .await
    }

    pub async fn session_connect(
        &self,
        session: &SessionResponse,
        xcloud_transfer_token: &str,
    ) -> Result<(), GssvApiError> {
        let resp = self
            .client
            .post(self.session_url(session, "/connect"))
            .json(&XCloudConnect {
                user_token: xcloud_transfer_token.into(),
            })
            .send()
            .await
            .map_err(GssvApiError::HttpError)?;

        match resp.status() {
            StatusCode::ACCEPTED => Ok(()),
            _ => Err(GssvApiError::Unknown),
        }
    }

    pub async fn get_session_state(
        &self,
        session: &SessionResponse,
    ) -> Result<SessionStateResponse, GssvApiError> {
        self.get_json(self.session_url(session, "/state"), None)
            .await
    }

    pub async fn get_session_config(
        &self,
        session: &SessionResponse,
    ) -> Result<GssvSessionConfig, GssvApiError> {
        self.get_json(self.session_url(session, "/configuration"), None)
            .await
    }

    pub async fn set_sdp(&self, session: &SessionResponse, sdp: &str) -> Result<(), GssvApiError> {
        let resp = self
            .client
            .post(self.session_url(session, "/sdp"))
            .json(&GssvSdpOffer {
                message_type: "offer".into(),
                sdp: sdp.to_string(),
                configuration: SdpConfiguration {
                    chat: ChannelVersion {
                        min_version: 1,
                        max_version: 1,
                    },
                    control: ChannelVersion {
                        min_version: 1,
                        max_version: 3,
                    },
                    input: ChannelVersion {
                        min_version: 1,
                        max_version: 7,
                    },
                    message: ChannelVersion {
                        min_version: 1,
                        max_version: 1,
                    },
                    audio: None,
                    video: None,
                    chat_configuration: ChatConfiguration {
                        bytes_per_sample: 2,
                        expected_clip_duration_ms: 100,
                        format: ChatAudioFormat {
                            codec: "opus".into(),
                            container: "webm".into(),
                        },
                        num_channels: 1,
                        sample_frequency_hz: 24000,
                    },
                },
            })
            .send()
            .await
            .map_err(GssvApiError::HttpError)?;

        match resp.status() {
            StatusCode::ACCEPTED => Ok(()),
            _ => Err(GssvApiError::Unknown),
        }
    }

    pub async fn set_ice(
        &self,
        session: &SessionResponse,
        ice: Vec<RTCIceCandidateInit>,
    ) -> Result<(), GssvApiError> {
        let resp = self
            .client
            .post(self.session_url(session, "/ice"))
            .json(&IceMessage {
                message_type: "iceCandidate".into(),
                candidate: ice,
            })
            .send()
            .await
            .map_err(GssvApiError::HttpError)?;

        match resp.status() {
            StatusCode::ACCEPTED => Ok(()),
            _ => Err(GssvApiError::Unknown),
        }
    }

    pub async fn get_sdp(
        &self,
        session: &SessionResponse,
    ) -> Result<SdpExchangeResponse, GssvApiError> {
        self.get_json(self.session_url(session, "/sdp"), None).await
    }

    pub async fn get_ice(
        &self,
        session: &SessionResponse,
    ) -> Result<IceExchangeResponse, GssvApiError> {
        self.get_json(self.session_url(session, "/ice"), None).await
    }

    pub async fn send_keepalive(
        &self,
        session: &SessionResponse,
    ) -> Result<KeepaliveResponse, GssvApiError> {
        self.client
            .post(self.session_url(session, "/keepalive"))
            .body("")
            .send()
            .await
            .map_err(GssvApiError::HttpError)?
            .json::<KeepaliveResponse>()
            .await
            .map_err(GssvApiError::HttpError)
    }
}

/* Requests */

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct LoginRequest {
    token: String,
    offering_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct XCloudConnect {
    user_token: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct GssvSessionSettings {
    nano_version: String,
    enable_text_to_speech: bool,
    high_contrast: u8,
    locale: String,
    use_ice_connection: bool,
    timezone_offset_minutes: u32,
    sdk_type: String,
    os_name: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GssvSessionConfig {
    title_id: String,
    system_update_group: String,
    settings: GssvSessionSettings,
    server_id: String,
    fallback_region_names: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ChannelVersion {
    min_version: u8,
    max_version: u8,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ChatAudioFormat {
    codec: String,
    container: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ChatConfiguration {
    bytes_per_sample: u8,
    expected_clip_duration_ms: u32,
    format: ChatAudioFormat,
    num_channels: u8,
    sample_frequency_hz: u32,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct SdpConfiguration {
    chat_configuration: ChatConfiguration,
    chat: ChannelVersion,
    control: ChannelVersion,
    input: ChannelVersion,
    message: ChannelVersion,
    #[serde(skip_serializing_if = "Option::is_none")]
    audio: Option<ChannelVersion>,
    #[serde(skip_serializing_if = "Option::is_none")]
    video: Option<ChannelVersion>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct GssvSdpOffer {
    message_type: String,
    // TODO: Create SDP model
    sdp: String,
    configuration: SdpConfiguration,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct IceMessage {
    message_type: String,
    candidate: Vec<RTCIceCandidateInit>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct AppEnvironment {
    client_app_id: String,
    client_app_type: String,
    client_app_version: String,
    client_sdk_version: String,
    http_environment: String,
    sdk_install_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct AppInfo {
    env: AppEnvironment,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct DevHardwareInfo {
    make: String,
    model: String,
    sdk_type: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct DevOsInfo {
    name: String,
    ver: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct DevDisplayDimensions {
    width_in_pixels: u16,
    height_in_pixels: u16,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct DevDisplayPixelDensity {
    dpi_x: u16,
    dpi_y: u16,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct DevDisplayInfo {
    dimensions: DevDisplayDimensions,
    pixel_density: DevDisplayPixelDensity,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct DevInfo {
    hw: DevHardwareInfo,
    os: DevOsInfo,
    display_info: DevDisplayInfo,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct DeviceInfo {
    app_info: AppInfo,
    dev: DevInfo,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct OfferingRegion {
    pub name: String,
    pub base_uri: String,
    pub network_test_hostname: Option<String>,
    pub is_default: bool,
    pub system_update_groups: Option<Vec<String>>,
    pub fallback_priority: i32,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct CloudEnvironment {
    pub name: String,
    pub auth_base_uri: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct ClientCloudSettings {
    pub environments: Vec<CloudEnvironment>,
}

/* Responses */
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct OfferingSettings {
    pub allow_region_selection: bool,
    pub regions: Vec<OfferingRegion>,
    pub selectable_server_types: Option<Vec<String>>,
    pub client_cloud_settings: ClientCloudSettings,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LoginResponse {
    pub offering_settings: OfferingSettings,
    pub market: String,
    pub gs_token: String,
    pub token_type: String,
    pub duration_in_seconds: u32,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ConsoleEntry {
    pub device_name: String,
    pub server_id: String,
    pub power_state: String,
    pub console_type: String,
    pub play_path: String,
    pub out_of_home_warning: bool,
    pub wireless_warning: bool,
    pub is_devkit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ConsolesResponse {
    pub total_items: u32,
    pub continuation_token: Option<String>,
    pub results: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TitleTab {
    pub id: String,
    pub tab_version: String,
    pub layout_version: String,
    pub manifest_version: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TitleDetails {
    pub product_id: String,
    pub xbox_title_id: String,
    pub has_entitlement: bool,
    pub blocked_by_family_safety: bool,
    pub supports_in_app_purchases: bool,
    pub supported_tabs: Option<TitleTab>,
    pub native_touch: bool,
    pub opt_out_of_default_layout_touch_controls: bool,
    pub programs: Vec<String>,
    pub is_free_in_store: bool,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TitleResult {
    pub title_id: String,
    pub details: TitleDetails,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TitlesResponse {
    pub e_tag: String,
    pub total_items: u32,
    pub results: Vec<TitleResult>,
    pub continuation_token: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SessionResponse {
    session_path: String,
}

pub enum SessionState {
    WaitingForResources,
    ReadyToConnect,
    Provisioning,
    Provisioned,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SessionStateResponse {
    pub state: String,
    pub error_details: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ChatConfigurationResponse {
    format: ChatAudioFormat,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SdpResponse {
    chat: u16,
    chat_configuration: ChatConfigurationResponse,
    control: u16,
    input: u16,
    message: u16,
    message_type: String,
    sdp: String,
    sdp_type: String,
    status: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SdpExchangeResponse {
    #[serde(with = "crate::serde_helpers::json_string")]
    pub exchange_response: SdpResponse,
    pub error_details: Option<String>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct IceExchangeResponse {
    #[serde(with = "crate::serde_helpers::json_string_ice_workaround")]
    pub exchange_response: Vec<RTCIceCandidateInit>,
    pub error_details: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct KeepaliveResponse {
    pub alive_seconds: Option<u32>,
    pub reason: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use webrtc::sdp::SessionDescription;

    fn sdp_offer_message() -> &'static str {
        r#"{"messageType":"offer","sdp":"v=0\r\no=- 3296606666082362637 2 IN IP4 127.0.0.1\r\ns=-\r\nt=0 0\r\na=group:BUNDLE 0 1 2\r\na=extmap-allow-mixed\r\na=msid-semantic: WMS\r\nm=audio 9 UDP/TLS/RTP/SAVPF 111 63 103 104 9 0 8 106 105 13 110 112 113 126\r\nc=IN IP4 0.0.0.0\r\na=rtcp:9 IN IP4 0.0.0.0\r\na=ice-ufrag:bSbi\r\na=ice-pwd:BXzujnFw/cHKF8tMgtoo/cne\r\na=ice-options:trickle\r\na=fingerprint:sha-256 CB:87:A2:17:63:29:8C:10:5F:CE:29:22:76:ED:C3:89:64:94:48:29:E0:7C:83:13:70:41:C0:5C:08:D2:69:33\r\na=setup:actpass\r\na=mid:0\r\na=extmap:1 urn:ietf:params:rtp-hdrext:ssrc-audio-level\r\na=extmap:2 http://www.webrtc.org/experiments/rtp-hdrext/abs-send-time\r\na=extmap:3 http://www.ietf.org/id/draft-holmer-rmcat-transport-wide-cc-extensions-01\r\na=extmap:4 urn:ietf:params:rtp-hdrext:sdes:mid\r\na=sendrecv\r\na=msid:- a75c2046-2efe-4b04-aeb9-ed7beecf7871\r\na=rtcp-mux\r\na=rtpmap:111 opus/48000/2\r\na=rtcp-fb:111 transport-cc\r\na=fmtp:111 minptime=10;useinbandfec=1\r\na=rtpmap:63 red/48000/2\r\na=fmtp:63 111/111\r\na=rtpmap:103 ISAC/16000\r\na=rtpmap:104 ISAC/32000\r\na=rtpmap:9 G722/8000\r\na=rtpmap:0 PCMU/8000\r\na=rtpmap:8 PCMA/8000\r\na=rtpmap:106 CN/32000\r\na=rtpmap:105 CN/16000\r\na=rtpmap:13 CN/8000\r\na=rtpmap:110 telephone-event/48000\r\na=rtpmap:112 telephone-event/32000\r\na=rtpmap:113 telephone-event/16000\r\na=rtpmap:126 telephone-event/8000\r\na=ssrc:2757659185 cname:8nJCvH9MPijHQSGZ\r\na=ssrc:2757659185 msid:- a75c2046-2efe-4b04-aeb9-ed7beecf7871\r\nm=video 9 UDP/TLS/RTP/SAVPF 96 97 98 99 100 101 102 122 127 121 125 107 108 109 124 120 123 119 35 36 37 38 39 40 41 42 114 115 116 43\r\nc=IN IP4 0.0.0.0\r\na=rtcp:9 IN IP4 0.0.0.0\r\na=ice-ufrag:bSbi\r\na=ice-pwd:BXzujnFw/cHKF8tMgtoo/cne\r\na=ice-options:trickle\r\na=fingerprint:sha-256 CB:87:A2:17:63:29:8C:10:5F:CE:29:22:76:ED:C3:89:64:94:48:29:E0:7C:83:13:70:41:C0:5C:08:D2:69:33\r\na=setup:actpass\r\na=mid:1\r\na=extmap:14 urn:ietf:params:rtp-hdrext:toffset\r\na=extmap:2 http://www.webrtc.org/experiments/rtp-hdrext/abs-send-time\r\na=extmap:13 urn:3gpp:video-orientation\r\na=extmap:3 http://www.ietf.org/id/draft-holmer-rmcat-transport-wide-cc-extensions-01\r\na=extmap:5 http://www.webrtc.org/experiments/rtp-hdrext/playout-delay\r\na=extmap:6 http://www.webrtc.org/experiments/rtp-hdrext/video-content-type\r\na=extmap:7 http://www.webrtc.org/experiments/rtp-hdrext/video-timing\r\na=extmap:8 http://www.webrtc.org/experiments/rtp-hdrext/color-space\r\na=extmap:4 urn:ietf:params:rtp-hdrext:sdes:mid\r\na=extmap:10 urn:ietf:params:rtp-hdrext:sdes:rtp-stream-id\r\na=extmap:11 urn:ietf:params:rtp-hdrext:sdes:repaired-rtp-stream-id\r\na=recvonly\r\na=rtcp-mux\r\na=rtcp-rsize\r\na=rtpmap:96 VP8/90000\r\na=rtcp-fb:96 goog-remb\r\na=rtcp-fb:96 transport-cc\r\na=rtcp-fb:96 ccm fir\r\na=rtcp-fb:96 nack\r\na=rtcp-fb:96 nack pli\r\na=rtpmap:97 rtx/90000\r\na=fmtp:97 apt=96\r\na=rtpmap:98 VP9/90000\r\na=rtcp-fb:98 goog-remb\r\na=rtcp-fb:98 transport-cc\r\na=rtcp-fb:98 ccm fir\r\na=rtcp-fb:98 nack\r\na=rtcp-fb:98 nack pli\r\na=fmtp:98 profile-id=0\r\na=rtpmap:99 rtx/90000\r\na=fmtp:99 apt=98\r\na=rtpmap:100 VP9/90000\r\na=rtcp-fb:100 goog-remb\r\na=rtcp-fb:100 transport-cc\r\na=rtcp-fb:100 ccm fir\r\na=rtcp-fb:100 nack\r\na=rtcp-fb:100 nack pli\r\na=fmtp:100 profile-id=2\r\na=rtpmap:101 rtx/90000\r\na=fmtp:101 apt=100\r\na=rtpmap:102 VP9/90000\r\na=rtcp-fb:102 goog-remb\r\na=rtcp-fb:102 transport-cc\r\na=rtcp-fb:102 ccm fir\r\na=rtcp-fb:102 nack\r\na=rtcp-fb:102 nack pli\r\na=fmtp:102 profile-id=1\r\na=rtpmap:122 rtx/90000\r\na=fmtp:122 apt=102\r\na=rtpmap:127 H264/90000\r\na=rtcp-fb:127 goog-remb\r\na=rtcp-fb:127 transport-cc\r\na=rtcp-fb:127 ccm fir\r\na=rtcp-fb:127 nack\r\na=rtcp-fb:127 nack pli\r\na=fmtp:127 level-asymmetry-allowed=1;packetization-mode=1;profile-level-id=42001f\r\na=rtpmap:121 rtx/90000\r\na=fmtp:121 apt=127\r\na=rtpmap:125 H264/90000\r\na=rtcp-fb:125 goog-remb\r\na=rtcp-fb:125 transport-cc\r\na=rtcp-fb:125 ccm fir\r\na=rtcp-fb:125 nack\r\na=rtcp-fb:125 nack pli\r\na=fmtp:125 level-asymmetry-allowed=1;packetization-mode=0;profile-level-id=42001f\r\na=rtpmap:107 rtx/90000\r\na=fmtp:107 apt=125\r\na=rtpmap:108 H264/90000\r\na=rtcp-fb:108 goog-remb\r\na=rtcp-fb:108 transport-cc\r\na=rtcp-fb:108 ccm fir\r\na=rtcp-fb:108 nack\r\na=rtcp-fb:108 nack pli\r\na=fmtp:108 level-asymmetry-allowed=1;packetization-mode=1;profile-level-id=42e01f\r\na=rtpmap:109 rtx/90000\r\na=fmtp:109 apt=108\r\na=rtpmap:124 H264/90000\r\na=rtcp-fb:124 goog-remb\r\na=rtcp-fb:124 transport-cc\r\na=rtcp-fb:124 ccm fir\r\na=rtcp-fb:124 nack\r\na=rtcp-fb:124 nack pli\r\na=fmtp:124 level-asymmetry-allowed=1;packetization-mode=0;profile-level-id=42e01f\r\na=rtpmap:120 rtx/90000\r\na=fmtp:120 apt=124\r\na=rtpmap:123 H264/90000\r\na=rtcp-fb:123 goog-remb\r\na=rtcp-fb:123 transport-cc\r\na=rtcp-fb:123 ccm fir\r\na=rtcp-fb:123 nack\r\na=rtcp-fb:123 nack pli\r\na=fmtp:123 level-asymmetry-allowed=1;packetization-mode=1;profile-level-id=4d001f\r\na=rtpmap:119 rtx/90000\r\na=fmtp:119 apt=123\r\na=rtpmap:35 H264/90000\r\na=rtcp-fb:35 goog-remb\r\na=rtcp-fb:35 transport-cc\r\na=rtcp-fb:35 ccm fir\r\na=rtcp-fb:35 nack\r\na=rtcp-fb:35 nack pli\r\na=fmtp:35 level-asymmetry-allowed=1;packetization-mode=0;profile-level-id=4d001f\r\na=rtpmap:36 rtx/90000\r\na=fmtp:36 apt=35\r\na=rtpmap:37 H264/90000\r\na=rtcp-fb:37 goog-remb\r\na=rtcp-fb:37 transport-cc\r\na=rtcp-fb:37 ccm fir\r\na=rtcp-fb:37 nack\r\na=rtcp-fb:37 nack pli\r\na=fmtp:37 level-asymmetry-allowed=1;packetization-mode=1;profile-level-id=f4001f\r\na=rtpmap:38 rtx/90000\r\na=fmtp:38 apt=37\r\na=rtpmap:39 H264/90000\r\na=rtcp-fb:39 goog-remb\r\na=rtcp-fb:39 transport-cc\r\na=rtcp-fb:39 ccm fir\r\na=rtcp-fb:39 nack\r\na=rtcp-fb:39 nack pli\r\na=fmtp:39 level-asymmetry-allowed=1;packetization-mode=0;profile-level-id=f4001f\r\na=rtpmap:40 rtx/90000\r\na=fmtp:40 apt=39\r\na=rtpmap:41 AV1/90000\r\na=rtcp-fb:41 goog-remb\r\na=rtcp-fb:41 transport-cc\r\na=rtcp-fb:41 ccm fir\r\na=rtcp-fb:41 nack\r\na=rtcp-fb:41 nack pli\r\na=rtpmap:42 rtx/90000\r\na=fmtp:42 apt=41\r\na=rtpmap:114 red/90000\r\na=rtpmap:115 rtx/90000\r\na=fmtp:115 apt=114\r\na=rtpmap:116 ulpfec/90000\r\na=rtpmap:43 flexfec-03/90000\r\na=rtcp-fb:43 goog-remb\r\na=rtcp-fb:43 transport-cc\r\na=fmtp:43 repair-window=10000000\r\nm=application 9 UDP/DTLS/SCTP webrtc-datachannel\r\nc=IN IP4 0.0.0.0\r\na=ice-ufrag:bSbi\r\na=ice-pwd:BXzujnFw/cHKF8tMgtoo/cne\r\na=ice-options:trickle\r\na=fingerprint:sha-256 CB:87:A2:17:63:29:8C:10:5F:CE:29:22:76:ED:C3:89:64:94:48:29:E0:7C:83:13:70:41:C0:5C:08:D2:69:33\r\na=setup:actpass\r\na=mid:2\r\na=sctp-port:5000\r\na=max-message-size:262144\r\n","configuration":{"chatConfiguration":{"bytesPerSample":2,"expectedClipDurationMs":20,"format":{"codec":"opus","container":"webm"},"numChannels":1,"sampleFrequencyHz":24000},"chat":{"minVersion":1,"maxVersion":1},"control":{"minVersion":1,"maxVersion":3},"input":{"minVersion":1,"maxVersion":7},"message":{"minVersion":1,"maxVersion":1}}}"#
    }

    fn sdp_response_message() -> &'static str {
        r#"{"exchangeResponse":"{\"chat\":1,\"chatConfiguration\":{\"format\":{\"codec\":\"opus\",\"container\":\"webm\"}},\"control\":3,\"input\":7,\"message\":1,\"messageType\":\"answer\",\"sdp\":\"v=0\\r\\no=- 1206897819200911867 2 IN IP4 127.0.0.1\\r\\ns=-\\r\\nt=0 0\\r\\na=group:BUNDLE 0 1 2\\r\\na=extmap-allow-mixed\\r\\na=msid-semantic: WMS 0 1\\r\\nm=audio 9 UDP/TLS/RTP/SAVPF 111 110\\r\\nc=IN IP4 0.0.0.0\\r\\na=rtcp:9 IN IP4 0.0.0.0\\r\\na=ice-ufrag:s1MX\\r\\na=ice-pwd:oG+NQK6nqS9svO3OnnXF6b9F\\r\\na=ice-options:trickle renomination\\r\\na=fingerprint:sha-256 4F:6B:3D:56:F5:CC:A5:D9:B2:63:85:DA:C1:23:90:C5:DB:9D:CF:01:3F:C0:B0:4A:3F:2A:33:09:94:1E:21:8A\\r\\na=setup:active\\r\\na=mid:0\\r\\na=extmap:1 urn:ietf:params:rtp-hdrext:ssrc-audio-level\\r\\na=extmap:2 http://www.webrtc.org/experiments/rtp-hdrext/abs-send-time\\r\\na=extmap:3 http://www.ietf.org/id/draft-holmer-rmcat-transport-wide-cc-extensions-01\\r\\na=extmap:4 urn:ietf:params:rtp-hdrext:sdes:mid\\r\\na=sendrecv\\r\\na=msid:0 f671d610-5792-4206-8b7a-5065f6c3b05f\\r\\na=rtcp-mux\\r\\na=rtpmap:111 opus/48000/2\\r\\na=fmtp:111 minptime=10;useinbandfec=1\\r\\na=rtpmap:110 telephone-event/48000\\r\\na=ssrc:1897225254 cname:OIHn/yQUJt/2NeUp\\r\\nm=video 9 UDP/TLS/RTP/SAVPF 127 121 125 107 108 109 124 120 123 119 114 115 116\\r\\nc=IN IP4 0.0.0.0\\r\\na=rtcp:9 IN IP4 0.0.0.0\\r\\na=ice-ufrag:s1MX\\r\\na=ice-pwd:oG+NQK6nqS9svO3OnnXF6b9F\\r\\na=ice-options:trickle renomination\\r\\na=fingerprint:sha-256 4F:6B:3D:56:F5:CC:A5:D9:B2:63:85:DA:C1:23:90:C5:DB:9D:CF:01:3F:C0:B0:4A:3F:2A:33:09:94:1E:21:8A\\r\\na=setup:active\\r\\na=mid:1\\r\\na=extmap:14 urn:ietf:params:rtp-hdrext:toffset\\r\\na=extmap:2 http://www.webrtc.org/experiments/rtp-hdrext/abs-send-time\\r\\na=extmap:13 urn:3gpp:video-orientation\\r\\na=extmap:3 http://www.ietf.org/id/draft-holmer-rmcat-transport-wide-cc-extensions-01\\r\\na=extmap:5 http://www.webrtc.org/experiments/rtp-hdrext/playout-delay\\r\\na=extmap:6 http://www.webrtc.org/experiments/rtp-hdrext/video-content-type\\r\\na=extmap:7 http://www.webrtc.org/experiments/rtp-hdrext/video-timing\\r\\na=extmap:8 http://www.webrtc.org/experiments/rtp-hdrext/color-space\\r\\na=extmap:4 urn:ietf:params:rtp-hdrext:sdes:mid\\r\\na=extmap:10 urn:ietf:params:rtp-hdrext:sdes:rtp-stream-id\\r\\na=extmap:11 urn:ietf:params:rtp-hdrext:sdes:repaired-rtp-stream-id\\r\\na=sendonly\\r\\na=msid:1 0f37d49d-f1ce-43e9-acfe-eabd60755d3f\\r\\na=rtcp-mux\\r\\na=rtcp-rsize\\r\\na=rtpmap:127 H264/90000\\r\\na=rtcp-fb:127 goog-remb\\r\\na=rtcp-fb:127 transport-cc\\r\\na=rtcp-fb:127 ccm fir\\r\\na=rtcp-fb:127 nack\\r\\na=rtcp-fb:127 nack pli\\r\\na=fmtp:127 level-asymmetry-allowed=1;packetization-mode=1;profile-level-id=42002a\\r\\na=rtpmap:121 rtx/90000\\r\\na=fmtp:121 apt=127\\r\\na=rtpmap:125 H264/90000\\r\\na=rtcp-fb:125 goog-remb\\r\\na=rtcp-fb:125 transport-cc\\r\\na=rtcp-fb:125 ccm fir\\r\\na=rtcp-fb:125 nack\\r\\na=rtcp-fb:125 nack pli\\r\\na=fmtp:125 level-asymmetry-allowed=1;packetization-mode=0;profile-level-id=42002a\\r\\na=rtpmap:107 rtx/90000\\r\\na=fmtp:107 apt=125\\r\\na=rtpmap:108 H264/90000\\r\\na=rtcp-fb:108 goog-remb\\r\\na=rtcp-fb:108 transport-cc\\r\\na=rtcp-fb:108 ccm fir\\r\\na=rtcp-fb:108 nack\\r\\na=rtcp-fb:108 nack pli\\r\\na=fmtp:108 level-asymmetry-allowed=1;packetization-mode=1;profile-level-id=42e02a\\r\\na=rtpmap:109 rtx/90000\\r\\na=fmtp:109 apt=108\\r\\na=rtpmap:124 H264/90000\\r\\na=rtcp-fb:124 goog-remb\\r\\na=rtcp-fb:124 transport-cc\\r\\na=rtcp-fb:124 ccm fir\\r\\na=rtcp-fb:124 nack\\r\\na=rtcp-fb:124 nack pli\\r\\na=fmtp:124 level-asymmetry-allowed=1;packetization-mode=0;profile-level-id=42e02a\\r\\na=rtpmap:120 rtx/90000\\r\\na=fmtp:120 apt=124\\r\\na=rtpmap:123 H264/90000\\r\\na=rtcp-fb:123 goog-remb\\r\\na=rtcp-fb:123 transport-cc\\r\\na=rtcp-fb:123 ccm fir\\r\\na=rtcp-fb:123 nack\\r\\na=rtcp-fb:123 nack pli\\r\\na=fmtp:123 level-asymmetry-allowed=1;packetization-mode=1;profile-level-id=4d002a\\r\\na=rtpmap:119 rtx/90000\\r\\na=fmtp:119 apt=123\\r\\na=rtpmap:114 red/90000\\r\\na=rtpmap:115 rtx/90000\\r\\na=fmtp:115 apt=114\\r\\na=rtpmap:116 ulpfec/90000\\r\\na=ssrc-group:FID 3945614638 633672403\\r\\na=ssrc:3945614638 cname:OIHn/yQUJt/2NeUp\\r\\na=ssrc:633672403 cname:OIHn/yQUJt/2NeUp\\r\\nm=application 9 UDP/DTLS/SCTP webrtc-datachannel\\r\\nc=IN IP4 0.0.0.0\\r\\nb=AS:30\\r\\na=ice-ufrag:s1MX\\r\\na=ice-pwd:oG+NQK6nqS9svO3OnnXF6b9F\\r\\na=ice-options:trickle renomination\\r\\na=fingerprint:sha-256 4F:6B:3D:56:F5:CC:A5:D9:B2:63:85:DA:C1:23:90:C5:DB:9D:CF:01:3F:C0:B0:4A:3F:2A:33:09:94:1E:21:8A\\r\\na=setup:active\\r\\na=mid:2\\r\\na=sctp-port:5000\\r\\na=max-message-size:262144\\r\\n\",\"sdpType\":\"answer\",\"status\":\"success\"}","errorDetails":null}"#
    }

    fn ice_request_message() -> &'static str {
        r#"{"messageType":"iceCandidate","candidate":[{"candidate":"candidate:3129489152 1 udp 2122260223 192.168.100.211 49254 typ host generation 0 ufrag bSbi network-id 1 network-cost 10","sdpMLineIndex":0,"sdpMid":"0"},{"candidate":"candidate:3129489152 1 udp 2122260223 192.168.100.211 55407 typ host generation 0 ufrag bSbi network-id 1 network-cost 10","sdpMLineIndex":1,"sdpMid":"1"},{"candidate":"candidate:3129489152 1 udp 2122260223 192.168.100.211 36059 typ host generation 0 ufrag bSbi network-id 1 network-cost 10","sdpMLineIndex":2,"sdpMid":"2"},{"candidate":"candidate:1504293356 1 udp 1686052607 111.243.105.102 49254 typ srflx raddr 192.168.100.211 rport 49254 generation 0 ufrag bSbi network-id 1 network-cost 10","sdpMLineIndex":0,"sdpMid":"0"},{"candidate":"candidate:1504293356 1 udp 1686052607 111.243.105.102 55407 typ srflx raddr 192.168.100.211 rport 55407 generation 0 ufrag bSbi network-id 1 network-cost 10","sdpMLineIndex":1,"sdpMid":"1"},{"candidate":"candidate:1504293356 1 udp 1686052607 111.243.105.102 36059 typ srflx raddr 192.168.100.211 rport 36059 generation 0 ufrag bSbi network-id 1 network-cost 10","sdpMLineIndex":2,"sdpMid":"2"},{"candidate":"candidate:4094413808 1 tcp 1518280447 192.168.100.211 9 typ host tcptype active generation 0 ufrag bSbi network-id 1 network-cost 10","sdpMLineIndex":0,"sdpMid":"0"},{"candidate":"candidate:4094413808 1 tcp 1518280447 192.168.100.211 9 typ host tcptype active generation 0 ufrag bSbi network-id 1 network-cost 10","sdpMLineIndex":1,"sdpMid":"1"},{"candidate":"candidate:4094413808 1 tcp 1518280447 192.168.100.211 9 typ host tcptype active generation 0 ufrag bSbi network-id 1 network-cost 10","sdpMLineIndex":2,"sdpMid":"2"}]}"#
    }

    fn ice_response_message() -> &'static str {
        r#"{"exchangeResponse":"[{\"candidate\":\"a=candidate:1 1 UDP 100 43.111.100.34 1136 typ host \",\"messageType\":\"iceCandidate\",\"sdpMLineIndex\":\"0\",\"sdpMid\":\"0\"},{\"candidate\":\"a=candidate:2 1 UDP 1 2603:1076:201:83::AB8:E9FE 9002 typ host \",\"messageType\":\"iceCandidate\",\"sdpMLineIndex\":\"0\",\"sdpMid\":\"0\"},{\"candidate\":\"a=end-of-candidates\",\"messageType\":\"iceCandidate\",\"sdpMLineIndex\":\"0\",\"sdpMid\":\"0\"}]","errorDetails":null}"#
    }

    #[test]
    fn deserialize_sdp_offer() {
        let data = sdp_offer_message();
        let json = serde_json::from_str::<GssvSdpOffer>(data);
        println!("{:?}", json);
        assert!(json.is_ok());

        let data = json.unwrap().sdp;
        let mut cursor = std::io::Cursor::new(&data);
        let result = SessionDescription::unmarshal(&mut cursor);
        println!("{:?}", result);
        assert!(result.is_ok())
    }

    #[test]
    fn deserialize_sdp_answer() {
        let data = sdp_response_message();
        let json = serde_json::from_str::<SdpExchangeResponse>(data);
        println!("{:?}", json);
        assert!(json.is_ok());

        let data = json.unwrap().exchange_response.sdp;
        let mut cursor = std::io::Cursor::new(&data);
        let result = SessionDescription::unmarshal(&mut cursor);
        println!("Deserialized={:?}", result);
        assert!(result.is_ok());
    }

    #[test]
    fn serialize_sdp_offer() {
        let data = sdp_offer_message();
        let json = serde_json::from_str::<GssvSdpOffer>(data);
        println!("{:?}", json);
        assert!(json.is_ok());

        let data = json.unwrap().sdp;
        let mut cursor = std::io::Cursor::new(&data);
        let result = SessionDescription::unmarshal(&mut cursor);
        assert!(result.is_ok());
        let serialized = result.unwrap().marshal();
        assert_eq!(serialized, data);
    }

    #[test]
    fn serialize_sdp_answer() {
        let data = sdp_response_message();
        let json = serde_json::from_str::<SdpExchangeResponse>(data);
        println!("{:?}", json);
        assert!(json.is_ok());

        let data = json.unwrap().exchange_response.sdp;
        let mut cursor = std::io::Cursor::new(&data);
        let result = SessionDescription::unmarshal(&mut cursor);
        assert!(result.is_ok());
        let serialized = result.unwrap().marshal();
        assert_eq!(serialized, data);
    }

    #[test]
    fn deserialize_ice_request() {
        let data = ice_request_message();
        let result = serde_json::from_str::<IceMessage>(&data);
        assert!(result.is_ok());
    }

    #[test]
    fn deserialize_ice_response() {
        let data = ice_response_message();
        let result = serde_json::from_str::<IceExchangeResponse>(&data);
        println!("{:?}", result);
        assert!(result.is_ok());
    }

    #[test]
    fn serialize_ice_request() {
        let data = ice_request_message();
        let result = serde_json::from_str::<IceMessage>(&data);
        assert!(result.is_ok());
        let serialized = serde_json::to_string(&result.unwrap());
        assert!(serialized.is_ok());
    }
}
