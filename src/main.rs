use ha_mitaffald::settings::Settings;
use ha_mitaffald::sync_data;

#[tokio::main]
async fn main() {
    let settings = Settings::new().expect("Failed to read settings");

    let report = sync_data(settings).await;

    if let Err(x) = report {
        eprintln!(
            "Failure while reporting data (some entities may have been updated): {}",
            x
        );
    }

    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
}
