pub mod api;
pub mod error;
mod packets;
mod serde_helpers;
use chrono::{Duration, Utc};

use api::{ConsolesResponse, SessionResponse, TitleResult};

use crate::api::GssvApi;
use crate::error::GsError;

pub struct GamestreamingClient {
    xhome: GssvApi,
    xcloud: GssvApi,
    transfer_token: String,
}

impl GamestreamingClient {
    const CONNECTION_TIMEOUT_SECS: i64 = 30;

    pub async fn create(gssv_token: &str, xcloud_transfer_token: &str) -> Result<Self, GsError> {
        Ok(Self {
            xhome: GssvApi::login_xcloud(gssv_token).await?,
            xcloud: GssvApi::login_xhome(gssv_token).await?,
            transfer_token: xcloud_transfer_token.into(),
        })
    }

    pub async fn lookup_games(&self) -> Result<Vec<TitleResult>, GsError> {
        Ok(self
            .xcloud
            .get_titles()
            .await
            .map_err(GsError::ApiError)?
            .results)
    }

    pub async fn lookup_consoles(&self) -> Result<ConsolesResponse, GsError> {
        self.xhome.get_consoles().await.map_err(GsError::ApiError)
    }

    async fn start_stream(
        &self,
        platform: &str,
        server_id: Option<&str>,
        title_id: Option<&str>,
    ) -> Result<SessionResponse, GsError> {
        let api = match platform {
            "home" => {
                if title_id.is_some() || server_id.is_none() {
                    return Err(GsError::Provisioning(
                        "Invalid params to home stream, no server id or title id was provided"
                            .into(),
                    ));
                }
                &self.xhome
            }
            "cloud" => {
                if server_id.is_some() || title_id.is_none() {
                    return Err(GsError::Provisioning(
                        "Invalid params to cloud stream, no title id or server id was provided"
                            .into(),
                    ));
                }
                &self.xcloud
            }
            v => {
                return Err(GsError::Provisioning(format!(
                    "Invalid platform, expected 'home' or 'cloud', got '{}'",
                    v
                )))
            }
        };

        let session = api.start_session(server_id, title_id).await?;
        let start_time = Utc::now();

        while Utc::now() - start_time
            < Duration::seconds(GamestreamingClient::CONNECTION_TIMEOUT_SECS)
        {
            match api.get_session_state(&session).await?.state.as_ref() {
                "WaitingForResources" | "Provisioning" => {
                    println!("Waiting for session to get ready");
                }
                "ReadyToConnect" => {
                    println!("Stream is ready to connect");
                    if let Err(connect_err) =
                        api.session_connect(&session, &self.transfer_token).await
                    {
                        println!("Failed to connect to session");
                        return Err(connect_err.into());
                    }
                }
                "Provisioned" => {
                    println!("Game session is ready!");
                    return Ok(session);
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
        self.start_stream("cloud", None, Some(title_id)).await
    }

    pub async fn start_stream_xhome(&self, server_id: &str) -> Result<SessionResponse, GsError> {
        self.start_stream("home", Some(server_id), None).await
    }

    pub async fn exchange_ice(&self) {}

    pub async fn exchange_sdp(&self) {}
}

#[cfg(test)]
mod tests {}
