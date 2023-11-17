use std::collections::HashMap;

use homeassistant::HASensor;
use mitaffald::get_containers;
use opentelemetry::trace::Tracer;
use rumqttc::Client;
use settings::Settings;
use tracing::{info, instrument, trace};

pub mod homeassistant;
pub mod mitaffald;
pub mod settings;

#[instrument(fields(cosmin = "boss"))]
pub fn sync_data(
    settings: Settings,
    sensor_map: &mut HashMap<String, HASensor>,
) -> Result<(), String> {
    let span = tracing::span!(tracing::Level::INFO, "my span");
    let entry = span.enter();
    info!("own span");
    drop(entry);

    // let tracer = opentelemetry::global::tracer("ex.com/basic");
    // tracer.in_span("sync_data_manual", |cx| {
    //     info!(target: "my-target", "hello from {}. My price is {}. I am also inside a Span!", "banana", 2.99);
    // });

    tracing::event!(tracing::Level::INFO, "inside my_function!");
    trace!("Starting sync_data");
    info!("Connecting to MQTT broker");
    info!(stg = ?settings, "Connecting to MQTT broker without referencing stg");
    info!(stg = ?settings, "Connecting to MQTT broker with referencing part of settings: {}", settings.mqtt.host);

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
