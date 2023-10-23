use ha_mitaffald::homeassistant::HASensor;
use ha_mitaffald::settings::Settings;
use ha_mitaffald::sync_data;
use std::collections::HashMap;

fn main() {
    let settings = Settings::new().expect("Failed to read settings");
    let mut sensor_map: HashMap<String, HASensor> = HashMap::new();

    let report = sync_data(settings, &mut sensor_map);

    if let Err(x) = report {
        eprintln!(
            "Failure while reporting data (some entities may have been updated): {}",
            x
        );
    }
}
