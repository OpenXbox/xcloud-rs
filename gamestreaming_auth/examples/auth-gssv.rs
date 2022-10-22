use gamestreaming_auth::authenticate;
// use std::fs::File;

const TOKEN_FILEPATH: &str = "tokens.json";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _streaming_tokens = authenticate(TOKEN_FILEPATH)
        .await
        .expect("Authentication failed!");

    /*
    let file_out = File::create("streaming_tokens.json")?;
    serde_json::to_writer_pretty(file_out, &streaming_tokens)?;
    */

    Ok(())
}