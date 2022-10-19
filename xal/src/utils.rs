use serde::{Deserialize, Serialize};
use std::fs;

use crate::models::response::{SisuAuthorizationResponse, XCloudTokenResponse, XSTSResponse};
// use crate::authenticator::SpecialTokenResponse;

#[derive(Serialize, Deserialize, Debug)]
pub struct TokenStore {
    // FIXME: Understand lifetimes... and probably get rid of 'static lifetime
    // on XalClientParameters
    // Finally: De-/serialize XalClientParameters
    // pub client_params: XalClientParameters,
    // pub wl_token: SpecialTokenResponse,
    pub sisu_tokens: SisuAuthorizationResponse,
    pub gssv_token: XSTSResponse,
    pub xcloud_transfer_token: XCloudTokenResponse,
}

impl TokenStore {
    pub fn load(filepath: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let s = fs::read_to_string(filepath)?;
        serde_json::from_str(&s).map_err(|e| e.into())
    }

    pub fn save(&self, filepath: &str) -> Result<(), Box<dyn std::error::Error>> {
        let s = serde_json::to_string_pretty(self)?;
        fs::write(filepath, s).map_err(|e| e.into())
    }
}
