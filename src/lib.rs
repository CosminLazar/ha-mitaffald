use std::collections::HashMap;

use homeassistant::HASensor;
use mitaffald::get_containers;
use rumqttc::Client;
use settings::Settings;

pub mod homeassistant;
pub mod mitaffald;
pub mod settings;

pub fn sync_data(
    settings: Settings,
    sensor_map: &mut HashMap<String, HASensor>,
) -> Result<(), String> {
    let (mut client, mut connection) = Client::new(settings.mqtt.into(), 200);
    let mut has_errors = false;

    get_containers(settings.affaldvarme)?
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
