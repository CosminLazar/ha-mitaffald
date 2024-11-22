mod mqtt;

use crate::mqtt::CollectingClient;
use ha_mitaffald::{
    mitaffald::settings::{Address, AddressId, AffaldVarmeConfig},
    settings::Settings,
    sync_data,
};
use rumqttc::Publish;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use testcontainers::{
    core::{ContainerPort, WaitFor},
    runners::AsyncRunner,
    GenericImage, ImageExt,
};
use url::Url;

#[tokio::test]
async fn smoke_test_insta() {
    // GenericImage::new(name, tag).with_network(network)
    let mqtt_server = GenericImage::new("hivemq/hivemq-ce", "latest")
        .with_exposed_port(ContainerPort::Tcp(1883))
        .with_wait_for(WaitFor::message_on_stdout("Started HiveMQ in"))
        .with_network("bridge")
        .start()
        .await
        .expect("Failed to start container, is Docker running?");

    let mqtt_server_port = mqtt_server
        .get_host_port_ipv4(1883)
        .await
        .expect("Failed to get port binding");

    let mut mit_affald_server = mockito::Server::new_async().await;
    let mit_affald_server_url = Url::parse(&mit_affald_server.url()).unwrap();
    let address_id = "123".to_string();
    let mit_affald_server = mit_affald_server
        .mock(
            "GET",
            format!("/api/calendar/address/{}", address_id).as_str(),
        )
        .with_status(200)
        .with_body_from_file("src/mitaffald/remote_responses/container_information.json")
        .create_async()
        .await;

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

    let mut home_assistant = CollectingClient::new();
    home_assistant.start(&settings.mqtt);

    let sync_result = sync_data(settings).await;

    assert!(
        sync_result.is_ok(),
        "Error synchronizing: {:?}",
        sync_result.err()
    );

    let ha_messages_result = home_assistant.wait_for_messages(20, Duration::from_secs(60));

    assert!(
        ha_messages_result.is_ok(),
        "Error waiting for messages: {}",
        ha_messages_result.unwrap_err()
    );

    mit_affald_server.assert_async().await;

    let actual = actual(ha_messages_result.unwrap());

    insta::with_settings!({
        filters=>vec![
            (r#"\\"last_update\\":\s*\\"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d+\+\d{2}:\d{2}\\""#,
            r#"\"last_update\": \"[REDACTED]\""#)
        ]
    }, {
        insta::assert_yaml_snapshot!(actual);
    });
}

fn actual(messages: Vec<Publish>) -> Vec<MqttMessage> {
    let mut x: Vec<MqttMessage> = messages
        .iter()
        .map(|message| {
            let topic = message.topic.clone();
            let payload = String::from_utf8(message.payload.to_vec()).unwrap();
            MqttMessage { topic, payload }
        })
        .collect();

    x.sort();
    x
}

#[derive(Serialize, Deserialize, PartialOrd, PartialEq, Eq, Ord)]
struct MqttMessage {
    topic: String,
    payload: String,
}
