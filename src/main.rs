use ha_mitaffald::settings::Settings;
use ha_mitaffald::sync_data;

#[tokio::main]
async fn main() {
    println!("Starting data synchronization");

    let settings = Settings::new().expect("Failed to read settings");
    let report = sync_data(settings).await;

    match report {
        Ok(_) => println!("Data synchronization completed"),
        Err(x) => eprintln!(
            "Data synchronization failed (some entities may have been updated), error: {}",
            x
        ),
    }
}
