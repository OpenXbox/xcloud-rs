use gamestreaming_webrtc::{GamestreamingClient, Platform};
use xal::utils::TokenStore;

const TOKENS_FILEPATH: &str = "tokens.json";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ts = match TokenStore::load(TOKENS_FILEPATH) {
        Ok(ts) => ts,
        Err(err) => {
            println!("Failed to load tokens!");
            return Err(err);
        }
    };

    let _ = GamestreamingClient::create(
        Platform::Cloud,
        &ts.gssv_token.token_data.token,
        &ts.xcloud_transfer_token.lpt,
    )
    .await?;

    todo!("Implement client");
}
