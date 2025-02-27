use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use taralli_primitives::{
    alloy::{network::Network, primitives::Address, providers::Provider, transports::Transport},
    systems::ProvingSystemId,
};
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationConfig {
    pub minimum_allowed_proving_time: u32,
    pub maximum_start_delay: u32,
    pub maximum_allowed_stake: u128,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            minimum_allowed_proving_time: 30,              // 30 secs
            maximum_start_delay: 300,                      // 5 min
            maximum_allowed_stake: 1000000000000000000000, // 1000 ether
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyzerConfig {
    pub market_address: Address,
    pub supported_proving_systems: Vec<ProvingSystemId>,
    pub validation: ValidationConfig,
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
    pub subscribed_to: u8,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            server_url: Url::parse("http://localhost:8000").unwrap(),
            request_timeout: 30,
            max_retries: 3,
            subscribed_to: 0xFF, // All bits set to 1, so all proving systems are subscribed to.
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
    phantom_data: PhantomData<(T, N)>,
}

impl<T, P, N> ProviderConfig<T, P, N>
where
    T: Transport + Clone,
    P: Provider<T, N> + Clone,
    N: Network + Clone,
{
    pub fn new(rpc_provider: P, market_address: Address, server_url: Url) -> Self {
        Self {
            rpc_provider,
            market_address,
            server_url,
            phantom_data: PhantomData,
        }
    }
}
