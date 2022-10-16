use gamestreaming_webrtc::api::GssvApi;
use std::io;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!(
        r#"!!! ACTION REQUIRED !!!
Paste the GSSV Token and hit [ENTER]"#
    );

    let mut gssv_token = String::new();
    let _ = io::stdin().read_line(&mut gssv_token)?;

    // Strip newline
    let gssv_token = gssv_token.strip_suffix('\n').unwrap();

    println!("Logging in");
    let home_api = GssvApi::login_xhome(gssv_token).await?;

    println!("Fetching consoles");
    let resp = home_api.get_consoles().await?;
    println!("{:?}", resp);

    Ok(())
}
