use crate::mitaffald::Container;
use crate::settings::MQTTConfig;
use rumqttc::{Client, LastWill, MqttOptions};

const HA_AVAILABILITY_TOPIC: &str = "garbage_bin/availability";

impl From<MQTTConfig> for MqttOptions {
    fn from(val: MQTTConfig) -> Self {
        let mut config = MqttOptions::new(val.client_id, val.host, val.port);
        config
            .set_credentials(val.username, val.password)
            .set_last_will(LastWill::new(
                HA_AVAILABILITY_TOPIC,
                "offline",
                rumqttc::QoS::AtLeastOnce,
                true,
            ));

        config
    }
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

//generate tests for this module
#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use testcontainers::{clients, core::WaitFor};

    #[test]
    fn smoke_test() {
        let docker = clients::Cli::default();
        let hive = docker.run(HiveMQContainer::default());
        let port = hive.get_host_port_ipv4(1883);

        println!("Ip address: {}", hive.get_bridge_ip_address());
        println!("HiveMQ is listening on port {}", port);
    }

    const NAME: &str = "hivemq/hivemq-ce";
    const TAG: &str = "latest";

    struct HiveMQContainer {
        _env_vars: HashMap<String, String>,
        tag: String,
    }

    impl Default for HiveMQContainer {
        fn default() -> Self {
            let mut env_vars = HashMap::new();
            env_vars.insert("discovery.type".to_owned(), "single-node".to_owned());
            HiveMQContainer {
                _env_vars: env_vars,
                tag: TAG.to_owned(),
            }
        }
    }

    impl testcontainers::Image for HiveMQContainer {
        type Args = ();

        fn name(&self) -> String {
            NAME.to_owned()
        }

        fn tag(&self) -> String {
            self.tag.to_owned()
        }

        fn ready_conditions(&self) -> Vec<testcontainers::core::WaitFor> {
            vec![WaitFor::message_on_stdout("Started HiveMQ in")]
        }

        fn expose_ports(&self) -> Vec<u16> {
            vec![1883]
        }

        // fn bla (&self){
        //     self.
        // }
    }
}
