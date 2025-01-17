use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use taralli_primitives::alloy::{
    network::Network, primitives::Address, providers::Provider, signers::Signer,
    transports::Transport,
};
use taralli_primitives::systems::ProvingSystemId;
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardTokenConfig {
    pub address: Address,
    pub decimal: u8,
}

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
pub struct RequesterConfig<T, P, N, S>
where
    T: Transport + Clone,
    P: Provider<T, N> + Clone,
    N: Network + Clone,
    S: Signer + Clone,
{
    pub rpc_provider: P,
    pub signer: S,
    pub market_address: Address,
    pub reward_token_config: RewardTokenConfig,
    pub validation: ValidationConfig,
    pub proving_system_id: ProvingSystemId,
    pub taralli_server_url: Url,
    phantom_data: PhantomData<(T, N)>,
}

impl<T, P, N, S> RequesterConfig<T, P, N, S>
where
    T: Transport + Clone,
    P: Provider<T, N> + Clone,
    N: Network + Clone,
    S: Signer + Clone,
{
    pub fn new(
        rpc_provider: P,
        signer: S,
        taralli_server_url: Url,
        market_address: Address,
        reward_token_address: Address,
        reward_token_decimals: u8,
        proving_system_id: ProvingSystemId,
    ) -> Self {
        let reward_token_config = RewardTokenConfig {
            address: reward_token_address,
            decimal: reward_token_decimals,
        };
        Self {
            rpc_provider,
            signer,
            taralli_server_url,
            market_address,
            reward_token_config,
            validation: ValidationConfig::default(),
            proving_system_id,
            phantom_data: PhantomData,
        }
    }

    pub fn with_validation(mut self, validation: ValidationConfig) -> Self {
        self.validation = validation;
        self
    }
}
