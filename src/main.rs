use ha_mitaffald::settings::Settings;
use ha_mitaffald::sync_data;

#[tokio::main]
async fn main() {
    loop {
        println!("Starting data synchronization");

        let settings = Settings::new().expect("Failed to read settings");
        let update_interval =
            tokio::time::Duration::from_secs(settings.update_interval_minutes * 60);

        let report = sync_data(settings).await;

        match report {
            Ok(_) => println!("Data synchronization completed"),
            Err(x) => eprintln!(
                "Data synchronization failed (some entities may have been updated), error: {}",
                x
            ),
        }

        println!(
            "Next synchronization scheduled at {}",
            (chrono::Local::now() + update_interval).format("%Y-%m-%d %H:%M:%S")
        );

        tokio::time::sleep(update_interval).await;
    }
}
