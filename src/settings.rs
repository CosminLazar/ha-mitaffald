use crate::mitaffald::settings::AffaldVarmeConfig;
use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct Settings {
    pub mqtt: MQTTConfig,
    pub affaldvarme: AffaldVarmeConfig,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let settings = Config::builder()
            .add_source(File::with_name("config/default.toml"))
            .add_source(File::with_name("config/secrets.toml").required(false))
            .add_source(Environment::default().separator("_"))
            .build()?;

        settings.try_deserialize()
    }
}

#[derive(Debug, Deserialize, Clone)]
#[allow(unused)]
pub struct MQTTConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub client_id: String,
}
