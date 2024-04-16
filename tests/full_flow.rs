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
use std::{collections::HashMap, iter::repeat};
use testcontainers::clients;
use url::Url;

#[tokio::test]
async fn smoke_test() {
    let docker = clients::Cli::default();
    let mqtt_server = docker.run(HiveMQContainer::default());
    let mqtt_server_port = mqtt_server.get_host_port_ipv4(1883);

    let mut mit_affald_server = mockito::Server::new();
    let mit_affald_server_url = Url::parse(&mit_affald_server.url()).unwrap();
    let address_id = "123".to_string();
    let mit_affald_server = mit_affald_server
        .mock(
            "GET",
            format!("/api/calendar/address/{}", address_id).as_str(),
        )
        .with_status(200)
        .with_body_from_file("src/mitaffald/remote_responses/container_information.json")
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

    let mut collecting_client = CollectingClient::new();
    collecting_client.start(&settings.mqtt);

    let mut sensor_map: HashMap<String, HASensor> = HashMap::new();
    let sync_result = sync_data(settings, &mut sensor_map).await;

    assert!(
        sync_result.is_ok(),
        "Error synchronizing: {:?}",
        sync_result.err()
    );

    let collect_result = collecting_client.wait_for_messages(27, Duration::from_secs(60));

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
                    //assert_json_include allows actual to contain more fields than expected (e.g. timestamps)
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
        "homeassistant/sensor/ha_affaldvarme_Madaffald/config",
        vec![r#"{
            "object_id": "ha_affaldvarme_Madaffald",
            "unique_id": "ha_affaldvarme_Madaffald",
            "name": "Madaffald",
            "state_topic": "garbage_bin/Madaffald/status",
            "json_attributes_topic": "garbage_bin/Madaffald/status",
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
        "garbage_bin/Madaffald/status",
        vec![
            r#"{
                "name": "Madaffald",
                "next_empty": "2024-04-26"
            }"#,
        ],
    );

    expectation.insert(
        "homeassistant/sensor/ha_affaldvarme_Pap/config",
        vec![
            r#"{
                "object_id": "ha_affaldvarme_Pap",
                "unique_id": "ha_affaldvarme_Pap",
                "name": "Pap",
                "state_topic": "garbage_bin/Pap/status",
                "json_attributes_topic": "garbage_bin/Pap/status",
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
}"#,
        ],
    );

    expectation.insert(
        "garbage_bin/Pap/status",
        vec![
            r#"{
                "name": "Pap",
                "next_empty": "2024-05-09"
            }"#,
        ],
    );

    expectation.insert(
        "homeassistant/sensor/ha_affaldvarme_Tekstiler/config",
        vec![
            r#"{
                "object_id": "ha_affaldvarme_Tekstiler",
                "unique_id": "ha_affaldvarme_Tekstiler",
                "name": "Tekstiler",
                "state_topic": "garbage_bin/Tekstiler/status",
                "json_attributes_topic": "garbage_bin/Tekstiler/status",
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
            }"#,
        ],
    );

    expectation.insert(
        "garbage_bin/Tekstiler/status",
        vec![
            r#"{
                "name": "Tekstiler",
                "next_empty": "2024-05-09"
            }"#,
        ],
    );

    expectation.insert(
        "homeassistant/sensor/ha_affaldvarme_Plast/config",
        vec![
            r#"{
                "object_id": "ha_affaldvarme_Plast",
                "unique_id": "ha_affaldvarme_Plast",
                "name": "Plast",
                "state_topic": "garbage_bin/Plast/status",
                "json_attributes_topic": "garbage_bin/Plast/status",
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
            }"#,
        ],
    );

    expectation.insert(
        "garbage_bin/Plast/status",
        vec![
            r#"{
                "name": "Plast",
                "next_empty": "2024-04-18"
            }"#,
        ],
    );

    expectation.insert(
        "homeassistant/sensor/ha_affaldvarme_Glas/config",
        vec![
            r#"{
                "object_id": "ha_affaldvarme_Glas",
                "unique_id": "ha_affaldvarme_Glas",
                "name": "Glas",
                "state_topic": "garbage_bin/Glas/status",
                "json_attributes_topic": "garbage_bin/Glas/status",
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
            }"#,
        ],
    );

    expectation.insert(
        "garbage_bin/Glas/status",
        vec![
            r#"{
                "name": "Glas",
                "next_empty": "2024-04-18"
            }"#,
        ],
    );

    expectation.insert(
        "homeassistant/sensor/ha_affaldvarme_Metal/config",
        vec![
            r#"{
                "object_id": "ha_affaldvarme_Metal",
                "unique_id": "ha_affaldvarme_Metal",
                "name": "Metal",
                "state_topic": "garbage_bin/Metal/status",
                "json_attributes_topic": "garbage_bin/Metal/status",
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
            }"#,
        ],
    );

    expectation.insert(
        "garbage_bin/Metal/status",
        vec![
            r#"{
                "name": "Metal",
                "next_empty": "2024-04-18"
            }"#,
        ],
    );

    expectation.insert(
        "homeassistant/sensor/ha_affaldvarme_Restaffald/config",
        vec![
            r#"{
                "object_id": "ha_affaldvarme_Restaffald",
                "unique_id": "ha_affaldvarme_Restaffald",
                "name": "Restaffald",
                "state_topic": "garbage_bin/Restaffald/status",
                "json_attributes_topic": "garbage_bin/Restaffald/status",
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
            }"#,
        ],
    );

    expectation.insert(
        "garbage_bin/Restaffald/status",
        vec![
            r#"{
                "name": "Restaffald",
                "next_empty": "2024-04-26"
            }"#,
        ],
    );

    expectation.insert(
        "homeassistant/sensor/ha_affaldvarme_Papir/config",
        vec![
            r#"{
                "object_id": "ha_affaldvarme_Papir",
                "unique_id": "ha_affaldvarme_Papir",
                "name": "Papir",
                "state_topic": "garbage_bin/Papir/status",
                "json_attributes_topic": "garbage_bin/Papir/status",
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
            }"#,
        ],
    );

    expectation.insert(
        "garbage_bin/Papir/status",
        vec![
            r#"{
                "name": "Papir",
                "next_empty": "2024-05-09"
            }"#,
        ],
    );

    expectation.insert(
        "homeassistant/sensor/ha_affaldvarme_Mad__og_drikkekartoner/config",
        vec![
            r#"{
                "object_id": "ha_affaldvarme_Mad__og_drikkekartoner",
                "unique_id": "ha_affaldvarme_Mad__og_drikkekartoner",
                "name": "Mad- og drikkekartoner",
                "state_topic": "garbage_bin/Mad__og_drikkekartoner/status",
                "json_attributes_topic": "garbage_bin/Mad__og_drikkekartoner/status",
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
            }"#,
        ],
    );

    expectation.insert(
        "garbage_bin/Mad__og_drikkekartoner/status",
        vec![
            r#"{
                "name": "Mad- og drikkekartoner",
                "next_empty": "2024-04-18"
            }"#,
        ],
    );

    //expectation.insert("garbage_bin/availability",  vec!["online", "online"]);
    expectation.insert(
        "garbage_bin/availability",
        repeat("online").take(9).collect(),
    );

    expectation
}
