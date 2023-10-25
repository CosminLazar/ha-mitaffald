// use derive_builder::Builder;
// use serde::{Deserialize, Serialize};

// #[derive(Debug, Default, Builder, Clone)]
// pub struct Device {
//     /// Webpage link to manage the configuration.
//     configuration_url: Option<String>,

//     /// List of connections of the device.
//     connections: Option<Vec<(String, String)>>,

//     /// Hardware version.
//     hw_version: Option<String>,

//     /// List of IDs that uniquely identify the device.
//     identifiers: Option<Vec<String>>,

//     /// Manufacturer of the device.
//     manufacturer: Option<String>,

//     /// Model of the device.
//     model: Option<String>,

//     /// Name of the device.
//     name: Option<String>,

//     /// Suggest an area if the device isn’t in one yet.
//     suggested_area: Option<String>,

//     /// Firmware version.
//     sw_version: Option<String>,

//     /// Identifier of a device that routes messages between this device and Home Assistant. Examples of such devices are hubs, or parent devices of a sub-device. This is used to show device topology in Home Assistant.
//     via_device: Option<String>,
// }

// /// MQTT sensor configuration.
// #[derive(Debug, Default, Builder)]
// pub struct Sensor {
//     /// The MQTT topic subscribed to receive sensor values.
//     state_topic: String,

//     /// A list of MQTT topics subscribed to receive availability updates.
//     availability: Option<Vec<String>>,

//     /// Represents the available state.
//     payload_available: Option<String>,

//     /// Represents the unavailable state.
//     payload_not_available: Option<String>,

//     /// An MQTT topic subscribed to receive availability updates.
//     topic: String,

//     /// Template to extract device’s availability from the topic.
//     value_template: Option<String>,

//     /// Controls the conditions to set the entity to available.
//     availability_mode: Option<String>,

//     /// Template to extract device’s availability from the availability_topic.
//     availability_template: Option<String>,

//     /// The MQTT topic subscribed to receive availability updates.
//     availability_topic: Option<String>,

//     /// Information about the device.
//     device: Option<Device>,

//     /// A link to the webpage that can manage the configuration of this device.
//     configuration_url: Option<String>,

//     /// Flag which defines if the entity should be enabled when first added.
//     enabled_by_default: Option<bool>,

//     /// Encoding of the payloads received.
//     encoding: Option<String>,

//     /// Category of the entity.
//     entity_category: Option<String>,

//     /// Defines the number of seconds after the sensor’s state expires.
//     expire_after: Option<i32>,

//     /// Sends update events even if the value hasn’t changed.
//     force_update: Option<bool>,

//     /// Icon for the entity.
//     icon: Option<String>,

//     /// Template to extract the JSON dictionary.
//     json_attributes_template: Option<String>,

//     /// Topic subscribed to receive a JSON dictionary payload.
//     json_attributes_topic: Option<String>,

//     /// Template to extract the last_reset.
//     last_reset_value_template: Option<String>,

//     /// Name of the MQTT sensor.
//     name: Option<String>,

//     /// Used for automatic generation of entity_id.
//     object_id: Option<String>,

//     /// Number of decimals used in the sensor’s state after rounding.
//     suggested_display_precision: Option<i32>,

//     /// Maximum QoS level to be used.
//     qos: Option<i32>,

//     /// State class of the sensor.
//     state_class: Option<String>,

//     /// Unique ID for this sensor.
//     unique_id: Option<String>,

//     /// Units of measurement of the sensor.
//     unit_of_measurement: Option<String>,
// }
