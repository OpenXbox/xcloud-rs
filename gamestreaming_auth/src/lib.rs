use serde::{Serialize, Deserialize};
pub use xal;
use xal::{
    oauth2::{Scope, TokenResponse}, response::XSTSToken, TokenStore, CliCallbackHandler, Flows, XalAuthenticator
};


// Custom JSON response body
#[derive(Debug, Serialize, Deserialize)]
pub struct XCloudTokenResponse {
    pub lpt: String,
    pub refresh_token: String,
    pub user_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GamestreamingAuthContext {
    pub gssv_token: XSTSToken,
    pub xcloud_transfer_token: XCloudTokenResponse,
}

pub async fn authenticate(tokens_filepath: &str) -> Result<GamestreamingAuthContext, Box<dyn std::error::Error>> {
    let ts = match TokenStore::load_from_file(tokens_filepath) {
        Ok(ts) => ts,
        Err(err) => {
            println!("Failed to load tokens! err={err}");
            let mut authenticator = XalAuthenticator::default();
            println!("Authenticating via SISU...");
            let mut ts = Flows::xbox_live_sisu_full_flow(
                &mut authenticator,
                CliCallbackHandler
            ).await?;

            println!("Saving new tokens...");
            ts.update_timestamp();
            ts.save_to_file(tokens_filepath)?;
            ts
        }
    };

    let mut authenticator: XalAuthenticator = ts.clone().into();

    // Get GSSV token
    let gssv_token = authenticator.get_xsts_token(
        ts.device_token.as_ref(),
        ts.title_token.as_ref(),
        ts.user_token.as_ref(),
        "http://gssv.xboxlive.com/"
    )
    .await?;
    println!("GSSV={gssv_token:?}");

    // Get XCloud transfer token
    let xcloud_transfer_token = authenticator.refresh_token_for_scope::<XCloudTokenResponse>(
        ts.live_token.refresh_token().unwrap(),
        vec![Scope::new("service::http://Passport.NET/purpose::PURPOSE_XBOX_CLOUD_CONSOLE_TRANSFER_TOKEN".into())]
    ).await?; 
    println!("XCLOUD_TRANSFER={xcloud_transfer_token:?}");

    Ok(GamestreamingAuthContext {
        gssv_token,
        xcloud_transfer_token,
    })
}