use gamestreaming_webrtc::GamestreamingClient;
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

    println!(
        r#"!!! ACTION REQUIRED !!!
Paste the XCloud Transfer Token and hit [ENTER]"#
    );

    let mut xcloud_transfer_token = String::new();
    let _ = io::stdin().read_line(&mut xcloud_transfer_token)?;
    // Strip newline
    let xcloud_transfer_token = xcloud_transfer_token.strip_suffix('\n').unwrap();

    let _ = GamestreamingClient::create(gssv_token, xcloud_transfer_token).await?;

    todo!("Implement client");
}
