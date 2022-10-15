use std::{str::FromStr, fmt::Display};

use reqwest::{header, header::HeaderMap, Client, ClientBuilder, Request, Response};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum GssvApiError {
    #[error(transparent)]
    HttpError(#[from] reqwest::Error),
    #[error("Unknown API error")]
    Unknown,
}

/// Gamestreaming API Client
struct GssvApi {
    client: Client,
}

impl GssvApi {
    pub fn new(gssv_token: &str) -> Self {
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
        }
    }

    pub async fn get_consoles(&self) -> Result<Response, GssvApiError> {
        self.client
            .get("https://uks.gssv-play-prodxhome.xboxlive.com/v6/servers/home")
            .send()
            .await
            .map_err(GssvApiError::HttpError)
    }

    pub async fn get_titles(&self) -> Result<Response, GssvApiError> {
        self.client
            .get("https://uks.gssv-play-prodxhome.xboxlive.com/v1/titles")
            .send()
            .await
            .map_err(GssvApiError::HttpError)
    }

    pub async fn start_session(&self, server_id: &str) -> Result<Response, GssvApiError> {
        let device_info = DeviceInfo {
            app_info: AppInfo {
                env: AppEnvironment {
                    client_app_id: "Microsoft.GamingApp",
                    client_app_type: "native",
                    client_app_version: "2203.1001.4.0",
                    client_sdk_version: "5.3.0",
                    http_environment: "prod",
                    sdk_install_id: "",
                }
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
            .post("https://uks.gssv-play-prodxhome.xboxlive.com/v5/sessions/home/play")
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
            .map_err(GssvApiError::HttpError)
    }

    pub async fn xcloud_connect(&self, user_token: &str) -> Result<Response, GssvApiError> {
        self.client
            .post("https://uks.gssv-play-prodxhome.xboxlive.com/v4/sessions/home/connect")
            .json(&XCloudConnect {
                user_token: user_token,
            })
            .send()
            .await
            .map_err(GssvApiError::HttpError)
    }

    pub async fn get_session_state(&self, session_id: &str) -> Result<Response, GssvApiError> {
        self.client
            .get(format!(
                "https://uks.gssv-play-prodxhome.xboxlive.com/v4/sessions/home/{session_id}/state"
            ))
            .send()
            .await
            .map_err(GssvApiError::HttpError)
    }

    pub async fn get_session_config(&self, session_id: &str) -> Result<Response, GssvApiError> {
        self.client.get(format!("https://uks.gssv-play-prodxhome.xboxlive.com/v4/sessions/home/{session_id}/configuration"))
            .send()
            .await
            .map_err(GssvApiError::HttpError)
    }

    pub async fn set_sdp(&self, session_id: &str, sdp: &str) -> Result<Response, GssvApiError> {
        self.client
            .post(format!(
                "https://uks.gssv-play-prodxhome.xboxlive.com/v4/sessions/home/{session_id}/sdp"
            ))
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
            .map_err(GssvApiError::HttpError)
    }

    pub async fn set_ice(&self, session_id: &str, ice: &str) -> Result<Response, GssvApiError> {
        self.client
            .post(format!(
                "https://uks.gssv-play-prodxhome.xboxlive.com/v4/sessions/home/{session_id}/ice"
            ))
            .json(&IceMessage {
                message_type: "iceCandidate",
                candidate: "todo".to_string(),
            })
            .send()
            .await
            .map_err(GssvApiError::HttpError)
    }

    pub async fn get_sdp(&self, session_id: &str) -> Result<Response, GssvApiError> {
        self.client
            .get(format!(
                "https://uks.gssv-play-prodxhome.xboxlive.com/v4/sessions/home/{session_id}/sdp"
            ))
            .send()
            .await
            .map_err(GssvApiError::HttpError)
    }

    pub async fn get_ice(&self, session_id: &str) -> Result<Response, GssvApiError> {
        self.client
            .get(format!(
                "https://uks.gssv-play-prodxhome.xboxlive.com/v4/sessions/home/{session_id}/ice"
            ))
            .send()
            .await
            .map_err(GssvApiError::HttpError)
    }

    pub async fn send_keepalive(&self, session_id: &str) -> Result<Response, GssvApiError> {
        self.client.post(format!("https://uks.gssv-play-prodxhome.xboxlive.com/v4/sessions/home/{session_id}/keepalive"))
            .body("")
            .send()
            .await
            .map_err(GssvApiError::HttpError)
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct XCloudConnect<'a> {
    user_token: &'a str,
}

#[derive(Serialize, Deserialize, Debug)]
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
struct GssvSessionConfig<'a> {
    title_id: &'a str,
    system_update_group: &'a str,
    settings: GssvSessionSettings<'a>,
    server_id: &'a str,
    fallback_region_names: Vec<&'a str>,
}

#[derive(Serialize, Deserialize, Debug)]
struct ChannelVersion {
    min_version: u8,
    max_version: u8,
}

#[derive(Serialize, Deserialize, Debug)]
struct ChatAudioFormat<'a> {
    codec: &'a str,
    container: &'a str,
}

#[derive(Serialize, Deserialize, Debug)]
struct ChatConfiguration<'a> {
    bytes_per_sample: u8,
    expected_clip_duration_ms: u32,
    format: ChatAudioFormat<'a>,
    num_channels: u8,
    sample_frequency_hz: u32,
}

#[derive(Serialize, Deserialize, Debug)]
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
struct GssvSdpOffer<'a> {
    message_type: &'a str,
    // TODO: Create SDP model
    sdp: String,
    configuration: SdpConfiguration<'a>,
}

#[derive(Serialize, Deserialize, Debug)]
struct IceMessage<'a> {
    message_type: &'a str,
    // TODO: Create ICE candidate model
    candidate: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct AppEnvironment<'a> {
    client_app_id: &'a str,
    client_app_type: &'a str,
    client_app_version: &'a str,
    client_sdk_version: &'a str,
    http_environment: &'a str,
    sdk_install_id: &'a str,
}

#[derive(Serialize, Deserialize, Debug)]
struct AppInfo<'a> {
    env: AppEnvironment<'a>,
}

#[derive(Serialize, Deserialize, Debug)]
struct DevHardwareInfo<'a> {
    make: &'a str,
    model: &'a str,
    sdk_type: &'a str,
}

#[derive(Serialize, Deserialize, Debug)]
struct DevOsInfo<'a> {
    name: &'a str,
    ver: &'a str,
}

#[derive(Serialize, Deserialize, Debug)]
struct DevDisplayDimensions {
    width_in_pixels: u16,
    height_in_pixels: u16,
}

#[derive(Serialize, Deserialize, Debug)]
struct DevDisplayPixelDensity {
    dpi_x: u16,
    dpi_y: u16,
}

#[derive(Serialize, Deserialize, Debug)]
struct DevDisplayInfo {
    dimensions: DevDisplayDimensions,
    pixel_density: DevDisplayPixelDensity,
}

#[derive(Serialize, Deserialize, Debug)]
struct DevInfo<'a> {
    hw: DevHardwareInfo<'a>,
    os: DevOsInfo<'a>,
    display_info: DevDisplayInfo,
}

#[derive(Serialize, Deserialize, Debug)]
struct DeviceInfo<'a> {
    app_info: AppInfo<'a>,
    dev: DevInfo<'a>,
}

#[cfg(test)]
mod tests {}
