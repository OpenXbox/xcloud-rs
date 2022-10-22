use xal::TokenStore;
use smartglass::SmartglassClient;

const TOKENS_FILEPATH: &str = "tokens.json";

#[tokio::main]
async fn main() -> Result <(), Box<dyn std::error::Error>> {
    let ts = TokenStore::load_from_file(TOKENS_FILEPATH)
        .expect("Failed to load tokens from file");
    let xsts = ts.authorization_token
        .expect("No XSTS token available");

    let mut client = SmartglassClient::new(xsts, None, None)?;
    let consoles = client.get_console_list().await?;

    println!("Found {} console(s)", consoles.result.len());
    for console in &consoles.result {
        println!("{console:?}");
    }

    let console = match consoles.result.get(0) {
        Some(console) => console,
        None => return Ok(()),
    };

    let apps = client.get_installed_apps(
        &console.storage_devices.as_ref().unwrap()[0].storage_device_id
    ).await?;

    println!("Found {} installed apps", apps.result.len());
    for app in apps.result {
        println!("{app:?}");
    }

    Ok(())
}