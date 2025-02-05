use alloy::primitives::Address;
use serde::Deserialize;
use std::fs;
use std::str::FromStr;
use taralli_primitives::validation::offer::OfferSpecificConfig;
use taralli_primitives::validation::request::RequestSpecificConfig;
use taralli_primitives::validation::{CommonValidationConfig, ValidationMetaConfig};
use thiserror::Error;
use tracing::Level;
use url::Url;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub server_port: u16,
    pub rpc_url: String,
    pub log_level: String,
    pub validation_timeout_seconds: u32,
    pub market_address: Address,
    pub common_validation_config: CommonValidationConfig,
    pub request_validation_config: RequestSpecificConfig,
    pub offer_validation_config: OfferSpecificConfig,
}

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Failed to read config file: {0}")]
    FileReadError(#[from] std::io::Error),
    #[error("Failed to parse JSON: {0}")]
    JsonParseError(#[from] serde_json::Error),
    #[error("Failed to parse URL: {0}")]
    UrlParseError(#[from] url::ParseError),
    #[error("Failed to parse log level: {0}")]
    LogLevelParseError(String),
    #[error("Failed to decode hex: {0}")]
    HexDecodeError(#[from] hex::FromHexError),
}

impl Config {
    pub fn from_file(path: &str) -> Result<Self, ConfigError> {
        let data = fs::read_to_string(path)?;
        let config: Config = serde_json::from_str(&data)?;
        Ok(config)
    }

    pub fn rpc_url(&self) -> Result<Url, ConfigError> {
        Url::parse(&self.rpc_url).map_err(ConfigError::from)
    }

    pub fn log_level(&self) -> Result<Level, ConfigError> {
        Level::from_str(&self.log_level)
            .map_err(|_| ConfigError::LogLevelParseError(self.log_level.clone()))
    }

    pub fn validation_meta_config(&self) -> ValidationMetaConfig {
        ValidationMetaConfig {
            common: self.common_validation_config.clone(),
            request: self.request_validation_config.clone(),
            offer: self.offer_validation_config.clone(),
        }
    }
}
