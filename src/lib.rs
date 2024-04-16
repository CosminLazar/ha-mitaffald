use std::collections::HashMap;

use homeassistant::HASensor;
use mitaffald::{get_containers, Container};
use rumqttc::AsyncClient;
use settings::Settings;

pub mod homeassistant;
pub mod mitaffald;
pub mod settings;

pub async fn sync_data(
    settings: Settings,
    sensor_map: &mut HashMap<String, HASensor>,
) -> Result<(), String> {
    let (mut client, mut connection) = AsyncClient::new(settings.mqtt.into(), 200);
    let mut has_errors = false;

    for container in get_containers(settings.affaldvarme)
        .await?
        .into_iter()
        .fold(
            HashMap::<String, Container>::new(),
            |mut accumulator, item| {
                match accumulator.entry(item.name.clone()) {
                    std::collections::hash_map::Entry::Occupied(mut existing) => {
                        if existing.get().date > item.date {
                            existing.insert(item);
                        }
                    }
                    std::collections::hash_map::Entry::Vacant(v) => {
                        v.insert(item);
                    }
                }

                accumulator
            },
        )
        .into_values()
    {
        let report_result = sensor_map
            .entry(container.name.clone())
            .or_insert_with(|| HASensor::new(&container))
            .report(container, &mut client)
            .await;

        has_errors = has_errors || report_result.is_err();
    }

    //calling disconnect() causes an error in the connection iterator
    if let Err(x) = client.disconnect().await {
        return Err(x.to_string());
    }

    //iterate the connection untill we hit the error generated by disconnect()
    loop {
        let notification = connection.poll().await;
        if notification.is_err() {
            break;
        }
    }

    if has_errors {
        Err("Failed to report all containers".into())
    } else {
        Ok(())
    }
}
