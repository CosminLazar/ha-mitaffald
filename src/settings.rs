use crate::mitaffald::settings::AffaldVarmeConfig;
use config::{Config, ConfigError, File};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct Settings {
    pub mqtt: MQTTConfig,
    pub affaldvarme: AffaldVarmeConfig,
    pub otlp: OtlpConfig,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let settings = Config::builder()
            .add_source(File::with_name("config/default.toml"))
            .add_source(File::with_name("config/secrets.toml"))
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

#[derive(Debug, Deserialize, Clone)]
#[allow(unused)]
pub struct OtlpConfig {
    pub user: String,
    pub password: String,
}
