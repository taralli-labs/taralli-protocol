use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use taralli_primitives::alloy::{
    network::Network, primitives::Address, providers::Provider, signers::Signer,
    transports::Transport,
};
use taralli_primitives::systems::ProvingSystemId;
use taralli_primitives::validation::request::RequestValidationConfig;
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardTokenConfig {
    pub address: Address,
    pub decimal: u8,
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
    pub validation_config: RequestValidationConfig,
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
            validation_config: RequestValidationConfig::default(),
            proving_system_id,
            phantom_data: PhantomData,
        }
    }

    pub fn with_validation_config(mut self, validation_config: RequestValidationConfig) -> Self {
        self.validation_config = validation_config;
        self
    }
}
