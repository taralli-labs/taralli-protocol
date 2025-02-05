use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use taralli_primitives::{
    alloy::{network::Network, primitives::Address, providers::Provider, transports::Transport},
    systems::ProvingSystemId,
    validation::request::RequestValidationConfig,
};
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyzerConfig {
    pub market_address: Address,
    pub validation_config: RequestValidationConfig,
}

impl Default for AnalyzerConfig {
    fn default() -> Self {
        Self {
            market_address: Address::ZERO, // Will be set from provider config
            validation_config: RequestValidationConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BidderConfig {
    pub market_address: Address,
    pub min_bid_delay: u64,
    pub max_bid_attempts: u32,
}

impl Default for BidderConfig {
    fn default() -> Self {
        Self {
            market_address: Address::ZERO, // Will be set from provider config
            min_bid_delay: 0,
            max_bid_attempts: 3,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolverConfig {
    pub market_address: Address,
    pub confirmation_blocks: u64,
}

impl Default for ResolverConfig {
    fn default() -> Self {
        Self {
            market_address: Address::ZERO, // Will be set from provider config
            confirmation_blocks: 1,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    pub server_url: Url,
    pub request_timeout: u64,
    pub max_retries: u32,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            server_url: Url::parse("http://localhost:8080").unwrap(),
            request_timeout: 30,
            max_retries: 3,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerConfig {
    pub supported_proving_systems: Vec<ProvingSystemId>,
    pub max_concurrent_jobs: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig<T, P, N> {
    pub rpc_provider: P,
    pub market_address: Address,
    pub server_url: Url,
    pub validation_config: RequestValidationConfig,
    phantom_data: PhantomData<(T, N)>,
}

impl<T, P, N> ProviderConfig<T, P, N>
where
    T: Transport + Clone,
    P: Provider<T, N> + Clone,
    N: Network + Clone,
{
    pub fn new(
        rpc_provider: P,
        market_address: Address,
        server_url: Url,
        validation_config: RequestValidationConfig,
    ) -> Self {
        Self {
            rpc_provider,
            market_address,
            server_url,
            validation_config,
            phantom_data: PhantomData,
        }
    }
}
