use config::{Config, File, ConfigError};
use serde::Deserialize;


#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct Settings {
    pub mqtt: MQTTConfig,
    pub affaldvarme: AffaldVarmeConfig
}

//implement new for Settings
impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let settings = Config::builder()
        .add_source(File::with_name("config/default.toml"))
        .add_source(File::with_name("config/secrets.toml"))
        .build()?;

        settings.try_deserialize()
    }
}


#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct MQTTConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub client_id: String    
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct AffaldVarmeConfig {
    pub address: Address    
}

//not really tested
#[derive(Deserialize, Debug)]
pub struct TraditionalAddress {
    pub street_name: String,
    pub street_no: String,
    pub postal_code: String,
    pub city: String,
}

#[derive(Deserialize, Debug)]
pub struct AddressId {
    pub id: String,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum Address {
    Id(AddressId),
    FullySpecified(TraditionalAddress),
}



