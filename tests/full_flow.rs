mod hivemq;
mod mqtt;

use crate::mqtt::CollectingClient;
use assert_json_diff::assert_json_include;
use fluent_asserter::{assert_that, create_asserter};
use ha_mitaffald::{
    homeassistant::HASensor,
    mitaffald::settings::{Address, AddressId, AffaldVarmeConfig},
    settings::Settings,
    sync_data,
};
use hivemq::HiveMQContainer;
use rumqttc::Publish;
use serde_json::Value;
use std::time::Duration;
use std::{collections::HashMap, time::SystemTime};
use testcontainers::{clients, Image, RunnableImage};
use url::Url;

#[test]
fn smoke_test() {
    let docker = clients::Cli::default();
    let image: RunnableImage<HiveMQContainer> = HiveMQContainer::default().into();
    let image = image.with_container_name(format!(
        "name{:?}",
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
    ));
    // let mqtt_server = docker.run(HiveMQContainer::default());
    let mqtt_server = docker.run(image);

    let mqtt_server_port = mqtt_server.get_host_port_ipv4(1883);
    println!("Running local MQTT server on port {}", mqtt_server_port);

    let mut mit_affald_server = mockito::Server::new();
    let mit_affald_server_url = Url::parse(&mit_affald_server.url()).unwrap();
    let address_id = "123".to_string();
    let mit_affald_server = mit_affald_server
        .mock(
            "GET",
            format!("/Adresse/VisAdresseInfo?address-selected-id={}", address_id).as_str(),
        )
        .with_status(200)
        .with_body_from_file("src/mitaffald/remote_responses/container_information.html")
        .create();

    let settings = Settings {
        affaldvarme: AffaldVarmeConfig {
            address: Address::Id(AddressId { id: address_id }),
            base_url: mit_affald_server_url,
        },
        mqtt: ha_mitaffald::settings::MQTTConfig {
            client_id: "test".to_string(),
            host: "localhost".to_string(),
            port: mqtt_server_port,
            username: "".to_owned(),
            password: "".to_owned(),
        },
    };

    let mut collecting_client = CollectingClient::new(&settings.mqtt);
    collecting_client.start();

    let mut sensor_map: HashMap<String, HASensor> = HashMap::new();
    let sync_result = sync_data(settings, &mut sensor_map);

    assert!(
        sync_result.is_ok(),
        "Error synchronizing: {:?}",
        sync_result.err()
    );

    let collect_result = collecting_client.wait_for_messages(6, Duration::from_secs(60));

    assert!(
        collect_result.is_ok(),
        "Error waiting for messages: {}",
        collect_result.unwrap_err()
    );

    mit_affald_server.assert();

    let actual = actual(collect_result.unwrap());
    let expected = expectations();

    expected.into_iter().for_each(|(key, expected)| {
        assert_that!(&actual).contains_key(&key.to_owned());
        let actual = actual.get(key).unwrap();
        assert_eq!(actual.len(), expected.len());

        expected.iter().zip(actual).for_each(|(expected, actual)| {
            let actual_json = serde_json::from_str::<Value>(actual);
            let expected_json = serde_json::from_str::<Value>(expected);

            match (actual_json, expected_json) {
                (Ok(actual), Ok(expected)) => {
                    assert_json_include!(actual: actual, expected: expected)
                }
                _ => assert_eq!(actual, expected),
            }
        });
    });
}

fn actual(messages: Vec<Publish>) -> HashMap<String, Vec<String>> {
    let mut actual: HashMap<String, Vec<String>> = HashMap::new();
    for message in messages {
        let topic = message.topic;
        let payload = String::from_utf8(message.payload.to_vec()).unwrap();

        actual.entry(topic).or_insert_with(Vec::new).push(payload);
    }

    actual
}

fn expectations() -> HashMap<&'static str, Vec<&'static str>> {
    let mut expectation: HashMap<&'static str, Vec<&'static str>> = HashMap::new();
    expectation.insert(
        "homeassistant/sensor/ha_affaldvarme_11064295/config",         
        vec![r#"{
            "object_id": "ha_affaldvarme_11064295",
            "unique_id": "ha_affaldvarme_11064295",
            "name": "Restaffald",
            "state_topic": "garbage_bin/11064295/status",
            "json_attributes_topic": "garbage_bin/11064295/status",
            "value_template": "{{ (strptime(value_json.next_empty, '%Y-%m-%d').date() - now().date()).days }}",
            "availability_topic": "garbage_bin/availability",
            "payload_available": "online",
            "payload_not_available": "offline",
            "unit_of_measurement": "days",
            "device": {
              "identifiers": [
                "ha_affaldvarme"
              ],
              "name": "Affaldvarme integration",
              "sw_version": "1.0",
              "model": "Standard",
              "manufacturer": "Your Garbage Bin Manufacturer"
            },              
            "icon": "mdi:recycle"
          }"#]);

    expectation.insert(
        "homeassistant/sensor/ha_affaldvarme_12019493/config", 
        vec![r#"{
        "object_id": "ha_affaldvarme_12019493",
        "unique_id": "ha_affaldvarme_12019493",
        "name": "Genanvendeligt affald (Glas plast metal og papir pap)",
        "state_topic": "garbage_bin/12019493/status",
        "json_attributes_topic": "garbage_bin/12019493/status",
        "value_template": "{{ (strptime(value_json.next_empty, '%Y-%m-%d').date() - now().date()).days }}",
        "availability_topic": "garbage_bin/availability",
        "payload_available": "online",
        "payload_not_available": "offline",
        "unit_of_measurement": "days",
        "device": {
          "identifiers": [
            "ha_affaldvarme"
          ],
          "name": "Affaldvarme integration",
          "sw_version": "1.0",
          "model": "Standard",
          "manufacturer": "Your Garbage Bin Manufacturer"
        },              
        "icon": "mdi:recycle"
      }"#]);

    expectation.insert(
        "garbage_bin/11064295/status",
        vec![
            r#" {
        "id": "11064295",
        "size": "240 L",
        "frequency": "1 gang på 2 uger",
        "name": "Restaffald",
        "next_empty": "2024-08-04"        
        }"#,
        ],
    );
    expectation.insert(
        "garbage_bin/12019493/status",
        vec![
            r#"  {
        "id": "12019493",
        "size": "240 L",
        "frequency": "1 gang på 4 uger",
        "name": "Genanvendeligt affald (Glas plast metal og papir pap)",
        "next_empty": "2024-08-03"        
        }"#,
        ],
    );
    expectation.insert("garbage_bin/availability", vec!["online", "online"]);

    expectation
}
