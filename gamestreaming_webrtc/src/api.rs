use std::{fmt::Display, str::FromStr, string::ParseError};

use reqwest::{
    header, header::HeaderMap, Client, ClientBuilder, IntoUrl, Request, Response, StatusCode, Url,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum GssvApiError {
    #[error(transparent)]
    HttpError(#[from] reqwest::Error),
    #[error("Unknown error")]
    Unknown,
}

/// Gamestreaming API Client
struct GssvApi {
    client: Client,
    base_url: Url,
}

impl GssvApi {
    pub fn new(base_url: Url, gssv_token: &str) -> Self {
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
            base_url: base_url,
        }
    }

    async fn login(
        login_url: &str,
        offering_id: &str,
        token: &str,
    ) -> Result<LoginResponse, GssvApiError> {
        let mut headers = HeaderMap::new();
        headers.insert("x-gssv-client", "XboxComBrowser".parse()?);

        let client = reqwest::Client::new();
        client.post(login_url)
            .headers(headers)
            .json(&LoginRequest {
                token: token,
                offering_id: "xhome",
            })
            .send()
            .await
            .map_err(GssvApiError::HttpError)?
            .json::<LoginResponse>()
            .await
            .map_err(GssvApiError::HttpError)
    }

    pub async fn login_xhome(token: &str) -> Result<LoginResponse, GssvApiError> {
        GssvApi::login(
            "https://xhome.gssv-play-prod.xboxlive.com/v2/login/user",
            "xhome",
            token,
        )
        .await
    }

    pub async fn login_xcloud(token: &str) -> Result<LoginResponse, GssvApiError> {
        GssvApi::login(
            "https://xgpuweb.gssv-play-prod.xboxlive.com/v2/login/user",
            "xgpuweb",
            token,
        )
        .await
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

    pub async fn get_consoles(&self) -> Result<ConsolesResponse, GssvApiError> {
        self.client
            .get(self.url("/v6/servers/home"))
            .send()
            .await
            .map_err(GssvApiError::HttpError)?
            .json::<ConsolesResponse>()
            .await
            .map_err(GssvApiError::HttpError)
    }

    pub async fn get_titles(&self) -> Result<TitlesResponse, GssvApiError> {
        self.client
            .get(self.url("/v1/titles"))
            .send()
            .await
            .map_err(GssvApiError::HttpError)?
            .json::<TitlesResponse>()
            .await
            .map_err(GssvApiError::HttpError)
    }

    pub async fn start_session(&self, server_id: &str) -> Result<SessionResponse, GssvApiError> {
        let device_info = DeviceInfo {
            app_info: AppInfo {
                env: AppEnvironment {
                    client_app_id: "Microsoft.GamingApp",
                    client_app_type: "native",
                    client_app_version: "2203.1001.4.0",
                    client_sdk_version: "5.3.0",
                    http_environment: "prod",
                    sdk_install_id: "",
                },
            },
            dev: DevInfo {
                hw: DevHardwareInfo {
                    make: "Micro-Star International Co., Ltd.",
                    model: "GS66 Stealth 10SGS",
                    sdk_type: "native",
                },
                os: DevOsInfo {
                    name: "Windows 10 Pro",
                    ver: "19041.1.amd64fre.vb_release.191206-1406",
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

        let mut headers = HeaderMap::new();
        headers.insert("X-MS-Device-Info", device_info.parse()?);
        headers.insert("User-Agent", device_info.parse()?);

        self.client
            .post(self.url("/v5/sessions/home/play"))
            .json(&GssvSessionConfig {
                title_id: "",
                system_update_group: "",
                server_id: server_id,
                fallback_region_names: vec![],
                settings: GssvSessionSettings {
                    nano_version: "V3;WebrtcTransport.dll",
                    enable_text_to_speech: false,
                    high_contrast: 0,
                    locale: "en-US",
                    use_ice_connection: false,
                    timezone_offset_minutes: 120,
                    sdk_type: "web",
                    os_name: "windows",
                },
            })
            .headers(headers)
            .send()
            .await
            .map_err(GssvApiError::HttpError)?
            .json::<SessionResponse>()
            .await
            .map_err(GssvApiError::HttpError)
    }

    pub async fn session_connect(
        &self,
        session: &SessionResponse,
        user_token: &str,
    ) -> Result<(), GssvApiError> {
        let resp = self
            .client
            .post(self.session_url(session, "/connect"))
            .json(&XCloudConnect {
                user_token: user_token,
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
        self.client
            .get(self.session_url(session, "/state"))
            .send()
            .await
            .map_err(GssvApiError::HttpError)?
            .json::<SessionStateResponse>()
            .await
            .map_err(GssvApiError::HttpError)
    }

    pub async fn get_session_config(
        &self,
        session: &SessionResponse,
    ) -> Result<Response, GssvApiError> {
        self.client
            .get(self.session_url(session, "/configuration"))
            .send()
            .await
            .map_err(GssvApiError::HttpError)
    }

    pub async fn set_sdp(&self, session: &SessionResponse, sdp: &str) -> Result<(), GssvApiError> {
        let resp = self
            .client
            .post(self.session_url(session, "/sdp"))
            .json(&GssvSdpOffer {
                message_type: "offer",
                sdp: "todo".to_string(),
                configuration: SdpConfiguration {
                    containerize_audio: false,
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
                            codec: "opus",
                            container: "webm",
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

    pub async fn set_ice(&self, session: &SessionResponse, ice: &str) -> Result<(), GssvApiError> {
        let resp = self
            .client
            .post(self.session_url(session, "/ice"))
            .json(&IceMessage {
                message_type: "iceCandidate",
                candidate: "todo".to_string(),
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
    ) -> Result<ExchangeResponse, GssvApiError> {
        self.client
            .get(self.session_url(session, "/sdp"))
            .send()
            .await
            .map_err(GssvApiError::HttpError)?
            .json::<ExchangeResponse>()
            .await
            .map_err(GssvApiError::HttpError)
    }

    pub async fn get_ice(
        &self,
        session: &SessionResponse,
    ) -> Result<ExchangeResponse, GssvApiError> {
        self.client
            .get(self.session_url(session, "/ice"))
            .send()
            .await
            .map_err(GssvApiError::HttpError)?
            .json::<ExchangeResponse>()
            .await
            .map_err(GssvApiError::HttpError)
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
struct LoginRequest<'a> {
    token: &'a str,
    offering_id: &'a str,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct XCloudConnect<'a> {
    user_token: &'a str,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct GssvSessionSettings<'a> {
    nano_version: &'a str,
    enable_text_to_speech: bool,
    high_contrast: u8,
    locale: &'a str,
    use_ice_connection: bool,
    timezone_offset_minutes: u32,
    sdk_type: &'a str,
    os_name: &'a str,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct GssvSessionConfig<'a> {
    title_id: &'a str,
    system_update_group: &'a str,
    settings: GssvSessionSettings<'a>,
    server_id: &'a str,
    fallback_region_names: Vec<&'a str>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ChannelVersion {
    min_version: u8,
    max_version: u8,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ChatAudioFormat<'a> {
    codec: &'a str,
    container: &'a str,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ChatConfiguration<'a> {
    bytes_per_sample: u8,
    expected_clip_duration_ms: u32,
    format: ChatAudioFormat<'a>,
    num_channels: u8,
    sample_frequency_hz: u32,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct SdpConfiguration<'a> {
    containerize_audio: bool,
    chat_configuration: ChatConfiguration<'a>,
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
struct GssvSdpOffer<'a> {
    message_type: &'a str,
    // TODO: Create SDP model
    sdp: String,
    configuration: SdpConfiguration<'a>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct IceMessage<'a> {
    message_type: &'a str,
    // TODO: Create ICE candidate model
    candidate: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct AppEnvironment<'a> {
    client_app_id: &'a str,
    client_app_type: &'a str,
    client_app_version: &'a str,
    client_sdk_version: &'a str,
    http_environment: &'a str,
    sdk_install_id: &'a str,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct AppInfo<'a> {
    env: AppEnvironment<'a>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct DevHardwareInfo<'a> {
    make: &'a str,
    model: &'a str,
    sdk_type: &'a str,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct DevOsInfo<'a> {
    name: &'a str,
    ver: &'a str,
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
struct DevInfo<'a> {
    hw: DevHardwareInfo<'a>,
    os: DevOsInfo<'a>,
    display_info: DevDisplayInfo,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct DeviceInfo<'a> {
    app_info: AppInfo<'a>,
    dev: DevInfo<'a>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct OfferingRegion {
    name: String,
    base_uri: String,
    network_test_hostname: String,
    is_default: bool,
    system_update_groups: Option<Vec<String>>,
    fallback_priority: i32,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct CloudEnvironment {
    name: String,
    auth_base_uri: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct ClientCloudSettings {
    environments: Vec<CloudEnvironment>,
}

/* Responses */
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct OfferingSettings {
    allow_region_selection: bool,
    regions: Vec<OfferingRegion>,
    selectable_server_types: Option<Vec<String>>,
    client_cloud_settings: ClientCloudSettings,
    market: String,
    gs_token: String,
    token_type: String,
    duration_in_seconds: u32,
}


#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct LoginResponse {
    offering_settings: OfferingSettings,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ConsolesResponse {
    dummy: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct TitleTab {
    id: String,
    tab_version: String,
    layout_version: String,
    manifest_version: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct TitleDetails {
    product_id: String,
    xbox_title_id: String,
    has_entitlement: bool,
    blocked_by_family_safety: bool,
    supports_in_app_purchases: bool,
    supported_tabs: Option<TitleTab>,
    native_touch: bool,
    opt_out_of_default_layout_touch_controls: bool,
    programs: Vec<String>,
    is_free_in_store: bool,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct TitleResult {
    title_id: String,
    details: TitleDetails,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct TitlesResponse {
    e_tag: String,
    total_items: u32,
    results: Vec<TitleResult>,
    continuation_token: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct SessionResponse {
    session_path: String,
}

enum SessionState {
    WaitingForResources,
    ReadyToConnect,
    Provisioning,
    Provisioned,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct SessionStateResponse {
    state: String,
    error_details: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ExchangeResponse {
    exchange_response: String,
    error_details: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct KeepaliveResponse {
    alive_seconds: Option<u32>,
    reason: String,
}

#[cfg(test)]
mod tests {}
