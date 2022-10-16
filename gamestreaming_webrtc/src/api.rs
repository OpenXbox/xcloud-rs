use reqwest::{header, header::HeaderMap, Client, ClientBuilder, Response, StatusCode, Url};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json;
use thiserror::Error;

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
struct GssvApi {
    client: Client,
    base_url: Url,
    platform: &'static str,
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
            base_url: base_url,
            platform: platform,
        }
    }

    async fn login(
        offering_id: &str,
        token: &str,
    ) -> Result<LoginResponse, GssvApiError> {
        let login_url = format!("https://{}.gssv-play-prod.xboxlive.com/v2/login/user", offering_id);
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
        let resp = GssvApi::login(
            "xhome",
            token,
        )
        .await?;

        Ok(Self::new(
            Url::parse(&resp.offering_settings.regions.first().unwrap().base_uri).unwrap(),
            &resp.gs_token,
            "home",
        ))
    }

    pub async fn login_xcloud(token: &str) -> Result<Self, GssvApiError> {
        let resp = GssvApi::login(
            "xgpuweb",
            token,
        )
        .await?;

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

    pub async fn start_session(&self, server_id: &str) -> Result<SessionResponse, GssvApiError> {
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
            title_id: "".into(),
            system_update_group: "".into(),
            server_id: server_id.into(),
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
        user_token: &str,
    ) -> Result<(), GssvApiError> {
        let resp = self
            .client
            .post(self.session_url(session, "/connect"))
            .json(&XCloudConnect {
                user_token: user_token.into(),
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

    pub async fn set_ice(&self, session: &SessionResponse, ice: &str) -> Result<(), GssvApiError> {
        let resp = self
            .client
            .post(self.session_url(session, "/ice"))
            .json(&IceMessage {
                message_type: "iceCandidate".into(),
                candidate: "todo".into(),
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
        self.get_json(self.session_url(session, "/sdp"), None).await
    }

    pub async fn get_ice(
        &self,
        session: &SessionResponse,
    ) -> Result<ExchangeResponse, GssvApiError> {
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
struct GssvSessionConfig {
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
struct ChatAudioFormat {
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
    containerize_audio: bool,
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
    // TODO: Create ICE candidate model
    candidate: String,
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
struct OfferingRegion {
    name: String,
    base_uri: String,
    network_test_hostname: Option<String>,
    is_default: bool,
    system_update_groups: Option<Vec<String>>,
    fallback_priority: i32,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct CloudEnvironment {
    name: String,
    auth_base_uri: Option<String>,
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
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct LoginResponse {
    offering_settings: OfferingSettings,
    market: String,
    gs_token: String,
    token_type: String,
    duration_in_seconds: u32,
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
