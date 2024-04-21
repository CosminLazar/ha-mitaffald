use std::collections::HashMap;

use crate::mitaffald::Container;
use crate::settings::MQTTConfig;
use rumqttc::{AsyncClient, LastWill, MqttOptions};
use serde_json::json;

const HA_AVAILABILITY_TOPIC: &str = "garbage_bin/availability";
const HA_PAYLOAD_AVAILABLE: &str = "online";
const HA_PAYLOAD_NOT_AVAILABLE: &str = "offline";

impl From<MQTTConfig> for MqttOptions {
    fn from(val: MQTTConfig) -> Self {
        let mut config = MqttOptions::new(val.client_id, val.host, val.port);
        config
            .set_credentials(val.username, val.password)
            .set_last_will(LastWill::new(
                HA_AVAILABILITY_TOPIC,
                HA_PAYLOAD_NOT_AVAILABLE,
                rumqttc::QoS::AtLeastOnce,
                true,
            ));

        config
    }
}

pub struct CreatedState;
pub struct InitializedState {
    sensors: HashMap<String, HASensor>,
}

pub struct HADevice<T> {
    state: T,
}

impl Default for HADevice<CreatedState> {
    fn default() -> Self {
        HADevice {
            state: CreatedState,
        }
    }
}

impl HADevice<CreatedState> {
    pub async fn initialize(
        mut self,
        client: &mut AsyncClient,
    ) -> Result<HADevice<InitializedState>, String> {
        self.register_device(client)
            .await
            .map_err(|e| e.to_string())?;

        self.register_device_availability(client)
            .await
            .map_err(|e| e.to_string())?;

        Ok(HADevice {
            state: InitializedState {
                sensors: HashMap::new(),
            },
        })
    }

    async fn register_device(
        &mut self,
        client: &mut AsyncClient,
    ) -> Result<(), rumqttc::ClientError> {
        let payload = json!(
            {
                "unique_id": "ha_affaldvarme_device",
                "name": "Affaldvarme Device",
                "state_topic": HA_AVAILABILITY_TOPIC,
                "availability_topic": HA_AVAILABILITY_TOPIC,
                "payload_available": HA_PAYLOAD_AVAILABLE,
                "payload_not_available": HA_PAYLOAD_NOT_AVAILABLE,
                "device": {
                    "identifiers": ["ha_affaldvarme"],
                    "name": "Affaldvarme integration",
                    "sw_version": "1.0",
                    "model": "Standard",
                    "manufacturer": "Your humble rust developer"
                }
            }
        );

        client
            .publish(
                "homeassistant/sensor/ha_affaldvarme_device/config",
                rumqttc::QoS::AtLeastOnce,
                true,
                serde_json::to_string(&payload).expect("Failed to serialize"),
            )
            .await
    }

    async fn register_device_availability(
        &mut self,
        client: &mut AsyncClient,
    ) -> Result<(), rumqttc::ClientError> {
        client
            .publish(
                HA_AVAILABILITY_TOPIC,
                rumqttc::QoS::AtLeastOnce,
                true,
                HA_PAYLOAD_AVAILABLE,
            )
            .await
    }
}

impl HADevice<InitializedState> {
    pub async fn report(
        &mut self,
        container: Container,
        client: &mut AsyncClient,
    ) -> Result<(), String> {
        let sensor_id = HASensor::generate_sensor_id(&container);
        self.state
            .sensors
            .entry(sensor_id.clone())
            .or_insert_with(|| HASensor::new(&container))
            .report(container, client)
            .await
            .map_err(|e| e.to_string())
    }
}

struct HASensor {
    container_id: String,
    configure_topic: String,
    state_topic: String,
    is_initialized: bool,
}

impl HASensor {
    pub fn new(container: &Container) -> Self {
        let container_id: String = Self::generate_sensor_id(container);

        Self {
            configure_topic: format!(
                "homeassistant/sensor/ha_affaldvarme_{}/config",
                container_id
            ),
            state_topic: format!("garbage_bin/{}/status", container_id),
            is_initialized: false,
            container_id,
        }
    }

    fn generate_sensor_id(container: &Container) -> String {
        container
            .name
            .clone()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '_' })
            .collect()
    }

    async fn report(
        &mut self,
        container: Container,
        client: &mut AsyncClient,
    ) -> Result<(), rumqttc::ClientError> {
        self.register_sensor(&container, client).await?;

        self.register_sensor_value(&container, client).await
    }

    async fn register_sensor(
        &mut self,
        container: &Container,
        client: &mut AsyncClient,
    ) -> Result<(), rumqttc::ClientError> {
        if self.is_initialized {
            return Ok(());
        }

        let payload = json!(
            {
                "object_id": format!("ha_affaldvarme_{}", self.container_id),
                "unique_id": format!("ha_affaldvarme_{}", self.container_id),
                "name": container.name,
                "state_topic": self.state_topic,
                "json_attributes_topic": self.state_topic,
                "value_template": "{{ (strptime(value_json.next_empty, '%Y-%m-%d').date() - now().date()).days }}",
                "availability_topic": HA_AVAILABILITY_TOPIC,
                "payload_available": HA_PAYLOAD_AVAILABLE,
                "payload_not_available": HA_PAYLOAD_NOT_AVAILABLE,
                "unit_of_measurement": "days",
                "device": {
                    "identifiers": ["ha_affaldvarme"]
                },
                "icon": "mdi:recycle"
            }
        );

        let publish_result = client
            .publish(
                &self.configure_topic,
                rumqttc::QoS::AtLeastOnce,
                false,
                serde_json::to_string(&payload).expect("Failed to serialize"),
            )
            .await;

        if publish_result.is_ok() {
            self.is_initialized = true;
        }

        publish_result
    }

    async fn register_sensor_value(
        &self,
        container: &Container,
        client: &mut AsyncClient,
    ) -> Result<(), rumqttc::ClientError> {
        let payload = json!(
            {
                "name": container.name,
                "next_empty": container.date.format("%Y-%m-%d").to_string(),
                "last_update": chrono::Local::now().to_rfc3339()
            }
        );

        client
            .publish(
                &self.state_topic,
                rumqttc::QoS::AtLeastOnce,
                false,
                serde_json::to_string(&payload).expect("Failed to serialize"),
            )
            .await
    }
}
