use gamestreaming_webrtc::api::GssvApi;
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

    println!("Logging in");
    let home_api = GssvApi::login_xhome(&ts.gssv_token.token_data.token).await?;

    println!("Fetching consoles");
    let resp = home_api.get_consoles().await?;
    println!("{:?}", resp);

    Ok(())
}
