pub mod api;
pub mod error;
mod packets;
mod sdp;
mod serde_helpers;
use std::str::FromStr;

use chrono::{Duration, Utc};

use api::{
    ConsolesResponse, IceExchangeResponse, SdpExchangeResponse, SessionResponse, TitleResult,
};
use sdp::SdpSessionDescription;
use webrtc::ice_transport::ice_candidate::RTCIceCandidateInit;
use webrtc::sdp::SessionDescription;

use crate::api::GssvApi;
use crate::error::GsError;

#[derive(Debug, Eq, PartialEq)]
pub enum Platform {
    Cloud,
    Home,
}

impl FromStr for Platform {
    type Err = GsError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let platform = match s.to_lowercase().as_ref() {
            "home" => Platform::Home,
            "cloud" => Platform::Cloud,
            v => return Err(GsError::InvalidPlatform(v.into())),
        };
        Ok(platform)
    }
}

impl ToString for Platform {
    fn to_string(&self) -> String {
        let str = match self {
            Platform::Cloud => "cloud",
            Platform::Home => "home",
        };
        str.to_owned()
    }
}

pub struct GamestreamingClient {
    api: GssvApi,
    transfer_token: String,
    platform: Platform,
}

impl GamestreamingClient {
    const CONNECTION_TIMEOUT_SECS: i64 = 30;

    pub async fn create(
        platform: Platform,
        gssv_token: &str,
        xcloud_transfer_token: &str,
    ) -> Result<Self, GsError> {
        Ok(Self {
            api: match platform {
                Platform::Cloud => GssvApi::login_xcloud(gssv_token).await?,
                Platform::Home => GssvApi::login_xhome(gssv_token).await?,
            },
            transfer_token: xcloud_transfer_token.into(),
            platform,
        })
    }

    pub async fn lookup_games(&self) -> Result<Vec<TitleResult>, GsError> {
        if self.platform != Platform::Cloud {
            return Err(GsError::InvalidPlatform(
                "Cannot fetch games for this platform".into(),
            ));
        }

        Ok(self
            .api
            .get_titles()
            .await
            .map_err(GsError::ApiError)?
            .results)
    }

    pub async fn lookup_consoles(&self) -> Result<ConsolesResponse, GsError> {
        if self.platform != Platform::Home {
            return Err(GsError::InvalidPlatform(
                "Cannot fetch consoles for this platform".into(),
            ));
        }
        self.api.get_consoles().await.map_err(GsError::ApiError)
    }

    async fn start_stream(
        &self,
        server_id: Option<&str>,
        title_id: Option<&str>,
    ) -> Result<SessionResponse, GsError> {
        let session = match self.platform {
            Platform::Cloud => match title_id {
                None => {
                    return Err(GsError::Provisioning(
                        "No title id provided to start stream".into(),
                    ));
                }
                title_id => self.api.start_session(None, title_id).await?,
            },
            Platform::Home => match server_id {
                None => {
                    return Err(GsError::Provisioning(
                        "No server id provided to start stream".into(),
                    ));
                }
                server_id => self.api.start_session(server_id, None).await?,
            },
        };

        let start_time = Utc::now();

        while Utc::now() - start_time
            < Duration::seconds(GamestreamingClient::CONNECTION_TIMEOUT_SECS)
        {
            match self.api.get_session_state(&session).await?.state.as_ref() {
                "WaitingForResources" | "Provisioning" => {
                    println!("Waiting for session to get ready");
                }
                "ReadyToConnect" => {
                    println!("Stream is ready to connect");
                    if let Err(connect_err) = self
                        .api
                        .session_connect(&session, &self.transfer_token)
                        .await
                    {
                        println!("Failed to connect to session");
                        return Err(connect_err.into());
                    }
                }
                "Provisioned" => {
                    println!("Game session is ready!");
                    return Ok(session);
                }
                "Failed" => {
                    println!("Failed to provision session");
                    return Err(GsError::Provisioning("Received failed state".into()));
                }
                unknown_state => {
                    return Err(GsError::Provisioning(format!(
                        "Unhandled state: {}",
                        unknown_state
                    )));
                }
            }
        }

        Err(GsError::Provisioning(
            "Timeout waiting for Provisioned state".into(),
        ))
    }

    pub async fn start_stream_xcloud(&self, title_id: &str) -> Result<SessionResponse, GsError> {
        if self.platform != Platform::Cloud {
            return Err(GsError::InvalidPlatform(
                "Attempted to start XCloud stream via Home API".into(),
            ));
        }
        self.start_stream(None, Some(title_id)).await
    }

    pub async fn start_stream_xhome(&self, server_id: &str) -> Result<SessionResponse, GsError> {
        if self.platform != Platform::Home {
            return Err(GsError::InvalidPlatform(
                "Attempted to start Home stream via XCloud API".into(),
            ));
        }
        self.start_stream(Some(server_id), None).await
    }

    pub async fn exchange_sdp(
        &self,
        session: &SessionResponse,
        sdp: SessionDescription,
    ) -> Result<SdpExchangeResponse, GsError> {
        self.api
            .set_sdp(session, &SdpSessionDescription(sdp).to_string())
            .await
            .map_err(GsError::ApiError)?;
        self.api.get_sdp(session).await.map_err(GsError::ApiError)
    }

    pub async fn exchange_ice(
        &self,
        session: &SessionResponse,
        ice_candidate_init: Vec<RTCIceCandidateInit>,
    ) -> Result<IceExchangeResponse, GsError> {
        self.api
            .set_ice(session, ice_candidate_init)
            .await
            .map_err(GsError::ApiError)?;
        self.api.get_ice(session).await.map_err(GsError::ApiError)
    }
}

#[cfg(test)]
mod tests {}
