use ha_mitaffald::settings::Settings;
use ha_mitaffald::sync_data;

#[tokio::main]
async fn main() {
    loop {
        println!("Starting data synchronization");

        let settings = Settings::new().expect("Failed to read settings");
        let report_interval = tokio::time::Duration::from_secs(settings.reporting_interval_secs);
        let report = sync_data(settings).await;

        match report {
            Ok(_) => println!("Data synchronization completed"),
            Err(x) => eprintln!(
                "Data synchronization failed (some entities may have been updated), error: {}",
                x
            ),
        }

        println!(
            "Next synchronization will take place at: {}",
            chrono::Local::now() + report_interval
        );

        tokio::time::sleep(report_interval).await;
    }
}
