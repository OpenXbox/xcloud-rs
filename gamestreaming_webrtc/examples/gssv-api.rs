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
    println!("{:?}", home_api.get_consoles().await);

    let xcloud_api = GssvApi::login_xcloud(&ts.gssv_token.token_data.token).await?;
    println!("Fetching titles");
    println!("{:?}", xcloud_api.get_titles().await);

    Ok(())
}
