use ha_mitaffald::settings::Settings;
use ha_mitaffald::sync_data;
use tracing::{error, info, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
    loop {
        info!("Starting data synchronization");

        let settings = Settings::new().expect("Failed to read settings");
        let update_interval =
            tokio::time::Duration::from_secs(settings.update_interval_minutes * 60);

        let report = sync_data(settings).await;

        match report {
            Ok(_) => info!("Data synchronization completed"),
            Err(x) => error!(
                "Data synchronization failed (some entities may have been updated), error: {}",
                x
            ),
        }

        info!(
            "Next synchronization scheduled at {}",
            (chrono::Local::now() + update_interval).format("%Y-%m-%d %H:%M:%S")
        );

        tokio::time::sleep(update_interval).await;
    }
}
