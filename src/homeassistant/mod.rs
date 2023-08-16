use crate::mitaffald::Container;
use crate::settings::MQTTConfig;
use rumqttc::{Client, Connection, LastWill, MqttOptions};
const HA_AVAILABILITY_TOPIC: &str = "garbage_bin/availability";

// pub struct HomeAssistantClient {
//     client: Client,
//     connection: Connection,
//     is_initialized: bool,
// }

impl Into<MqttOptions> for MQTTConfig {
    fn into(self) -> MqttOptions {
        let mut config = MqttOptions::new(self.client_id, self.host, self.port);
        config
            .set_credentials(self.username, self.password)
            .set_last_will(LastWill::new(
                HA_AVAILABILITY_TOPIC,
                "offline",
                rumqttc::QoS::AtLeastOnce,
                true,
            ));

        config
    }
}

// trait PollToCompletion {
//     fn complete(&mut self);
// }

// impl PollToCompletion for Client{
//     fn complete(&mut self) {
//         self.
//     }
// }

// impl HomeAssistantClient {
//     pub fn new(config: MQTTConfig) -> Self {
//         let (client, connection) = Client::new(config.into(), 20);

//         Self {
//             client,
//             connection,
//             is_initialized: false,
//         }
//     }

// pub fn report(&mut self, container: &Container) {
//     let topic_config = HomeAssistantClient::get_sensor_topic(container);

//     //ERROR! only registers one sensor
//     if !self.is_initialized {
//         self.register_sensor(container, &topic_config);
//         self.register_sensor_availability();
//         self.is_initialized = true;
//     }

//     self.register_sensor_value(container, &topic_config);
// }

/// Asks for a disconnection from the MQTT broker and waits for the connection to be closed, while calling the connection iterator in order to make progress.
/// All data that is in the process of being sent will be sent before the disconnection is completed.
// pub fn disconnect(&mut self) {
//     //calling disconnect() causes an error in the connection iterator
//     self.client.disconnect().expect("Failed to disconnect");

//     //iterate the connection untill we hit the above generated error
//     let _ = self.connection.iter().take_while(|x| x.is_ok()).count();
// }

// fn register_sensor_value(&mut self, container: &Container, topic_config: &HomeAssistantSensor) {
//     let payload = format!(
//         r#"
//         {{
//         "id": "{id}",
//         "size": "{size}",
//         "frequency": "{frequency}",
//         "name": "{sensor_name}",
//         "next_empty": "{next_empty}",
//         "last_update": "{last_update}"
//         }}"#,
//         id = container.id,
//         size = container.size,
//         frequency = container.frequency,
//         sensor_name = container.name,
//         next_empty = container.get_next_empty().format("%Y-%m-%d"),
//         last_update = chrono::Local::now().to_rfc3339(),
//     );

//     let _r = self.client.publish(
//         &topic_config.state_topic,
//         rumqttc::QoS::AtLeastOnce,
//         false,
//         payload,
//     );
// }

// fn get_sensor_topic(container: &Container) -> HomeAssistantSensor {
//     let state_topic = format!("garbage_bin/{}/status", container.id);
//     let configure_topic = format!(
//         "homeassistant/sensor/ha_affaldvarme_{}/config",
//         container.id
//     );
//     HomeAssistantSensor {
//         state_topic,
//         configure_topic,
//     }
// }
//}

struct HomeAssistantSensor {
    state_topic: String,
    configure_topic: String,
}

pub struct HASensor {
    pub container_id: String,
    configure_topic: String,
    state_topic: String,
    is_initialized: bool,
}

impl HASensor {
    pub fn new(container: &Container) -> Self {
        Self {
            container_id: container.id.clone(),
            configure_topic: format!(
                "homeassistant/sensor/ha_affaldvarme_{}/config",
                container.id
            ),
            state_topic: format!("garbage_bin/{}/status", container.id),
            is_initialized: false,
        }
    }

    pub fn report(
        &mut self,
        container: Container,
        client: &mut Client,
    ) -> Result<(), rumqttc::ClientError> {
        if !self.is_initialized {
            self.register_sensor(&container, client)?;
            self.register_sensor_availability(client)?;
            self.is_initialized = true;
        }

        self.register_sensor_value(&container, client)?;
        Ok(())
    }

    fn register_sensor(
        &mut self,
        container: &Container,
        client: &mut Client,
    ) -> Result<(), rumqttc::ClientError> {
        let payload = format!(
            r#"
            {{
              "object_id": "ha_affaldvarme_{id}",
              "unique_id": "ha_affaldvarme_{id}",
              "name": "{sensor_name}",
              "state_topic": "{state_topic}",
              "json_attributes_topic": "{state_topic}",
              "value_template": "{{{{ (strptime(value_json.next_empty, '%Y-%m-%d').date() - now().date()).days }}}}",
              "availability_topic": "{availability_topic}",
              "payload_available": "online",
              "payload_not_available": "offline",
              "unit_of_measurement": "days",
              "device": {{
                "identifiers": [
                  "ha_affaldvarme"
                ],
                "name": "Affaldvarme integration",
                "sw_version": "1.0",
                "model": "Standard",
                "manufacturer": "Your Garbage Bin Manufacturer"
              }},              
              "icon": "mdi:recycle"
            }}"#,
            sensor_name = container.name,
            state_topic = self.state_topic,
            availability_topic = HA_AVAILABILITY_TOPIC,
            id = container.id,
        );

        client.publish(
            &self.configure_topic,
            rumqttc::QoS::AtLeastOnce,
            false,
            payload,
        )
    }

    fn register_sensor_availability(
        &self,
        client: &mut Client,
    ) -> Result<(), rumqttc::ClientError> {
        client.publish(
            HA_AVAILABILITY_TOPIC,
            rumqttc::QoS::AtLeastOnce,
            true,
            "online",
        )
    }

    fn register_sensor_value(
        &self,
        container: &Container,
        client: &mut Client,
    ) -> Result<(), rumqttc::ClientError> {
        let payload = format!(
            r#"
            {{
            "id": "{id}",
            "size": "{size}",
            "frequency": "{frequency}",
            "name": "{sensor_name}",
            "next_empty": "{next_empty}",
            "last_update": "{last_update}"
            }}"#,
            id = container.id,
            size = container.size,
            frequency = container.frequency,
            sensor_name = container.name,
            next_empty = container.get_next_empty().format("%Y-%m-%d"),
            last_update = chrono::Local::now().to_rfc3339(),
        );

        client.publish(&self.state_topic, rumqttc::QoS::AtLeastOnce, false, payload)
    }
}

//can we use asref here?

// impl Into<HASensor> for Container {
//     fn into(&self) -> HASensor {
//         HASensor::new(&self)
//     }
// }
