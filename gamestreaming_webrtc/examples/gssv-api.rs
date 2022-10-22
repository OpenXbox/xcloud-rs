use gamestreaming_webrtc::api::GssvApi;
use gamestreaming_auth::authenticate;

const TOKENS_FILEPATH: &str = "tokens.json";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let auth_ctx = authenticate(TOKENS_FILEPATH).await?;

    println!("Logging in");
    let home_api = GssvApi::login_xhome(&auth_ctx.gssv_token.token).await?;
    println!("Fetching consoles");
    println!("{:?}", home_api.get_consoles().await?);

    let xcloud_api = GssvApi::login_xcloud(&auth_ctx.gssv_token.token).await?;
    println!("Fetching titles");
    println!("{:?}", xcloud_api.get_titles().await?);

    Ok(())
}
