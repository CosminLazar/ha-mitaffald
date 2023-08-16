use std::collections::HashMap;
use ha_mitaffald::{get_containers, HASensor, Settings};
use rumqttc::Client;

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

fn sync_data(settings: Settings, sensor_map: &mut HashMap<String, HASensor>) -> Result<(), String> {
    let (mut client, mut connection) = Client::new(settings.mqtt.into(), 200);
    let mut has_errors = false;

    get_containers(settings.affaldvarme.address)?
        .into_iter()
        .for_each(|x| {
            let report_result = sensor_map
                .entry(x.id.clone())
                .or_insert_with(|| HASensor::new(&x))
                .report(x, &mut client);

            has_errors = has_errors || report_result.is_err();
        });

    //calling disconnect() causes an error in the connection iterator
    if let Err(x) = client.disconnect() {
        return Err(x.to_string());
    }

    //create own error and provide conversion from this?
    //client.disconnect()?;

    //iterate the connection untill we hit the above generated error
    connection.iter().take_while(|x| x.is_ok()).count();

    if has_errors {
        Err("Failed to report all containers".into())
    } else {
        Ok(())
    }
}
