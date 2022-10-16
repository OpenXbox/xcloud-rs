pub mod api;
pub mod error;
mod packets;
mod serde_helpers;

use api::{TitleResult, ConsolesResponse};

use crate::api::GssvApi;
use crate::error::GsError;

pub struct GamestreamingClient {
    xhome: GssvApi,
    xcloud: GssvApi,
}

impl GamestreamingClient {
    fn new(xhome: GssvApi, xcloud: GssvApi) -> Self {
        Self {
            xhome: xhome,
            xcloud: xcloud,
        }
    }

    pub async fn create(gssv_token: &str) -> Result<Self, GsError> {
        Ok(Self::new(
            GssvApi::login_xcloud(gssv_token).await?,
            GssvApi::login_xhome(gssv_token).await?,
        ))
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
        self.xhome
            .get_consoles()
            .await
            .map_err(GsError::ApiError)
    }

    pub async fn start_stream(&self, target: &str) {}

    async fn exchange_ice(&self) {}

    async fn exchange_sdp(&self) {}
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
