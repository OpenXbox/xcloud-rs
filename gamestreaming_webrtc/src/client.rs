use std::str::FromStr;

use chrono::{Duration, Utc};

use crate::api::GssvApi;
use crate::api::{
    ConsolesResponse, IceCandidate, IceExchangeResponse, SdpExchangeResponse, SessionResponse,
    TitleResult,
};
use crate::error::GsError;

#[derive(Debug, Clone, Eq, PartialEq)]
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

#[derive(Debug, Clone)]
pub struct GamestreamingClient {
    api: GssvApi,
    transfer_token: String,
    platform: Platform,
}

impl GamestreamingClient {
    const CONNECTION_TIMEOUT_SECS: i64 = 30;

    pub  fn create(
        platform: Platform,
        gssv_token: &str,
        xcloud_transfer_token: &str,
    ) -> Result<Self, GsError> {
        Ok(Self {
            api: match platform {
                Platform::Cloud => GssvApi::login_xcloud(gssv_token)?,
                Platform::Home => GssvApi::login_xhome(gssv_token)?,
            },
            transfer_token: xcloud_transfer_token.into(),
            platform,
        })
    }

    pub  fn lookup_games(&self) -> Result<Vec<TitleResult>, GsError> {
        if self.platform != Platform::Cloud {
            return Err(GsError::InvalidPlatform(
                "Cannot fetch games for this platform".into(),
            ));
        }

        Ok(self
            .api
            .get_titles()
            
            .map_err(GsError::ApiError)?
            .results)
    }

    pub  fn lookup_consoles(&self) -> Result<ConsolesResponse, GsError> {
        if self.platform != Platform::Home {
            return Err(GsError::InvalidPlatform(
                "Cannot fetch consoles for this platform".into(),
            ));
        }
        self.api.get_consoles().map_err(GsError::ApiError)
    }

     fn start_stream(
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
                title_id => self.api.start_session(None, title_id)?,
            },
            Platform::Home => match server_id {
                None => {
                    return Err(GsError::Provisioning(
                        "No server id provided to start stream".into(),
                    ));
                }
                server_id => self.api.start_session(server_id, None)?,
            },
        };

        let start_time = Utc::now();

        while Utc::now() - start_time
            < Duration::seconds(GamestreamingClient::CONNECTION_TIMEOUT_SECS)
        {
            let state_response = self.api.get_session_state(&session)?;
            match state_response.state.as_ref() {
                "WaitingForResources" | "Provisioning" => {
                    println!("Waiting for session to get ready");
                }
                "ReadyToConnect" => {
                    println!("Stream is ready to connect");
                    if let Err(connect_err) = self
                        .api
                        .session_connect(&session, &self.transfer_token)
                        
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
                    return Err(GsError::Provisioning(format!(
                        "Received failed state - error: {:?}",
                        state_response.error_details
                    )));
                }
                unknown_state => {
                    return Err(GsError::Provisioning(format!(
                        "Unhandled state: {} - error: {:?}",
                        unknown_state, state_response.error_details
                    )));
                }
            }

            self.lookup_games();
            //std::thread::sleep(std::time::Duration::from_secs(1));
        }

        Err(GsError::Provisioning(
            "Timeout waiting for Provisioned state".into(),
        ))
    }

    pub  fn start_stream_xcloud(&self, title_id: &str) -> Result<SessionResponse, GsError> {
        if self.platform != Platform::Cloud {
            return Err(GsError::InvalidPlatform(
                "Attempted to start XCloud stream via Home API".into(),
            ));
        }
        self.start_stream(None, Some(title_id))
    }

    pub  fn start_stream_xhome(&self, server_id: &str) -> Result<SessionResponse, GsError> {
        if self.platform != Platform::Home {
            return Err(GsError::InvalidPlatform(
                "Attempted to start Home stream via XCloud API".into(),
            ));
        }
        self.start_stream(Some(server_id), None)
    }

    pub  fn exchange_sdp(
        &self,
        session: &SessionResponse,
        sdp: &str,
    ) -> Result<SdpExchangeResponse, GsError> {
        self.api
            .set_sdp(session, sdp)
            
            .map_err(GsError::ApiError)?;
        let sdp_response = self.api.get_sdp(session).map_err(GsError::ApiError)?;
        let error_str = match &sdp_response.exchange_response.status {
            Some(status) => match status.as_ref() {
                "success" => {
                    return Ok(sdp_response);
                }
                _ => format!("Answer status != success => {:?}", sdp_response),
            },
            _ => {
                format!("SDP answer contains no status => {:?}", sdp_response)
            }
        };

        Err(GsError::ConnectionExchange(format!(
            "SDP failed, message={}",
            error_str
        )))
    }

    pub  fn exchange_ice(
        &self,
        session: &SessionResponse,
        ice_candidate_init: Vec<IceCandidate>,
    ) -> Result<IceExchangeResponse, GsError> {
        self.api
            .set_ice(session, ice_candidate_init)
            
            .map_err(GsError::ApiError)?;
        self.api.get_ice(session).map_err(GsError::ApiError)
    }
}

#[cfg(test)]
mod tests {}
