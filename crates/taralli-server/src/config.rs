use alloy::primitives::{Address, U256};
use serde::Deserialize;
use taralli_primitives::intents::{ComputeOffer, ComputeRequest};
use taralli_primitives::systems::System;
use taralli_primitives::validation::offer::OfferValidationConfig;
use taralli_primitives::validation::request::RequestValidationConfig;
use std::fs;
use std::str::FromStr;
use taralli_primitives::validation::{BaseValidationConfig, Validate};
use thiserror::Error;
use tracing::Level;
use url::Url;

#[derive(Debug, Deserialize)]
pub struct RawValidationConfig {
    pub common_validation_config: BaseValidationConfig,
    pub request_validation_config: RawRequestConfig,
    pub offer_validation_config: RawOfferConfig,
}

#[derive(Debug, Deserialize)]
pub struct RawRequestConfig {
    pub maximum_allowed_stake: u128,
}

#[derive(Debug, Deserialize)]
pub struct RawOfferConfig {
    pub maximum_allowed_reward: String,
    pub minimum_allowed_stake: String,
}

#[derive(Clone)]
pub struct ServerValidationConfigs {
    pub request: RequestValidationConfig,
    pub offer: OfferValidationConfig,
}

// derive type specific config from compute intent
pub trait ServerConfigProvider: Validate {
    fn get_config(configs: &ServerValidationConfigs) -> &Self::Config;
}

impl<S: System> ServerConfigProvider for ComputeRequest<S> {
    fn get_config(configs: &ServerValidationConfigs) -> &Self::Config {
        &configs.request
    }
}

impl<S: System> ServerConfigProvider for ComputeOffer<S> {    
    fn get_config(configs: &ServerValidationConfigs) -> &Self::Config {
        &configs.offer
    }
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub server_port: u16,
    pub rpc_url: String,
    pub log_level: String,
    pub validation_timeout_seconds: u32,
    pub market_address: Address,
    pub base_validation_config: BaseValidationConfig,
    pub request_validation_config: RawRequestConfig,
    pub offer_validation_config: RawOfferConfig,
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

    pub fn get_request_validation_config(&self) -> RequestValidationConfig {
        RequestValidationConfig {
            base: self.base_validation_config.clone(),
            maximum_allowed_stake: self.request_validation_config.maximum_allowed_stake,
        }
    }

    pub fn get_offer_validation_config(&self) -> OfferValidationConfig {
        OfferValidationConfig {
            base: self.base_validation_config.clone(),
            maximum_allowed_reward: U256::from_str(&self.offer_validation_config.maximum_allowed_reward)
                .expect("Invalid maximum_allowed_reward"),
            minimum_allowed_stake: U256::from_str(&self.offer_validation_config.minimum_allowed_stake)
                .expect("Invalid minimum_allowed_stake"),
        }
    }

    pub fn get_validation_configs(&self) -> ServerValidationConfigs {
        ServerValidationConfigs {
            request: self.get_request_validation_config(),
            offer: self.get_offer_validation_config(),
        }
    }
}
