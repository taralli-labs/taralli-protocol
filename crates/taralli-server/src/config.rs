use alloy::primitives::Address;
use serde::Deserialize;
use std::fs;
use std::str::FromStr;
use thiserror::Error;
use tracing::Level;
use url::Url;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub server_port: u16,
    pub admin_port: Option<u16>,
    pub rpc_url: String,
    pub log_level: String,
    pub validation_timeout_seconds: u32,
    pub minimum_allowed_proving_time: u32,
    pub maximum_allowed_start_delay: u32,
    pub maximum_allowed_stake: u128,
    pub market_address: Address,
    pub proving_system_ids: Vec<String>,
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
}
