use serde_json::Value;
use taralli_primitives::abi::universal_bombetta::UniversalBombetta::ProofRequest;
use taralli_primitives::alloy::{
    network::Network,
    primitives::U256,
    primitives::{Address, Bytes, B256},
    providers::Provider,
    transports::Transport,
};
use taralli_primitives::intents::request::ComputeRequest;
use taralli_primitives::systems::{SystemId, SystemParams};
use taralli_primitives::validation::request::RequestValidationConfig;

use super::{BaseIntentBuilder, IntentBuilder};
use crate::error::Result;
use crate::nonce_manager::Permit2NonceManager;

#[derive(Clone)]
pub struct ComputeRequestBuilder<T, P, N>
where
    T: Transport + Clone + Send + Sync,
    P: Provider<T, N> + Clone + Send + Sync,
    N: Network + Clone + Send + Sync,
{
    pub base: BaseIntentBuilder<T, P, N>,
    // Compute request specific params
    pub max_reward_amount: U256,
    pub min_reward_amount: U256,
    pub minimum_stake: u128,
    // Compute request validation config
    pub validation_config: RequestValidationConfig,
}

impl<T, P, N> ComputeRequestBuilder<T, P, N>
where
    T: Transport + Clone + Send + Sync,
    P: Provider<T, N> + Clone + Send + Sync,
    N: Network + Clone + Send + Sync,
{
    pub fn new(
        rpc_provider: P,
        signer_address: Address,
        market_address: Address,
        system_id: SystemId,
        validation_config: RequestValidationConfig,
    ) -> Self {
        // build permit2 nonce manager
        let permit2_nonce_manager = Permit2NonceManager::new(rpc_provider.clone(), signer_address);
        let base = BaseIntentBuilder {
            rpc_provider,
            permit2_nonce_manager,
            signer_address,
            auction_length: 0u32,
            market_address,
            nonce: U256::ZERO,
            reward_token_address: Address::ZERO,
            reward_token_decimals: 0u8,
            start_auction_timestamp: 0u64,
            end_auction_timestamp: 0u64,
            proving_time: 0u32,
            inputs_commitment: B256::ZERO,
            extra_data: Bytes::from(""),
            system_id,
            system: Value::Null,
            inputs: vec![],
        };
        Self {
            base,
            max_reward_amount: U256::ZERO,
            min_reward_amount: U256::ZERO,
            minimum_stake: 0u128,
            validation_config,
        }
    }

    pub async fn set_new_nonce(mut self) -> Result<Self> {
        self.base = self.base.set_new_nonce().await?;
        Ok(self)
    }

    pub async fn set_auction_timestamps_from_auction_length(mut self) -> Result<Self> {
        self.base = self
            .base
            .set_auction_timestamps_from_auction_length()
            .await?;
        Ok(self)
    }

    pub fn set_time_params(
        mut self,
        start_auction_ts: u64,
        end_auction_ts: u64,
        proving_time: u32,
    ) -> Self {
        self.base = self
            .base
            .set_time_params(start_auction_ts, end_auction_ts, proving_time);
        self
    }

    pub fn set_verification_commitment_params(
        mut self,
        inputs_commitment: B256,
        extra_data: Bytes,
    ) -> Self {
        self.base = self
            .base
            .set_verification_commitment_params(inputs_commitment, extra_data);
        self
    }

    pub fn auction_length(mut self, auction_length: u32) -> Self {
        self.base = self.base.auction_length(auction_length);
        self
    }

    pub fn market_address(mut self, market_address: Address) -> Self {
        self.base = self.base.market_address(market_address);
        self
    }

    pub fn nonce(mut self, nonce: U256) -> Self {
        self.base = self.base.nonce(nonce);
        self
    }

    pub fn reward_token_address(mut self, token_address: Address) -> Self {
        self.base = self.base.reward_token_address(token_address);
        self
    }

    pub fn reward_token_decimals(mut self, token_decimals: u8) -> Self {
        self.base.reward_token_decimals = token_decimals;
        self
    }

    pub fn start_auction_timestamp(mut self, timestamp: u64) -> Self {
        self.base = self.base.start_auction_timestamp(timestamp);
        self
    }

    pub fn end_auction_timestamp(mut self, timestamp: u64) -> Self {
        self.base = self.base.end_auction_timestamp(timestamp);
        self
    }

    pub fn proving_time(mut self, seconds_to_prove: u32) -> Self {
        self.base = self.base.proving_time(seconds_to_prove);
        self
    }

    pub fn extra_data(mut self, extra_data: Bytes) -> Self {
        self.base = self.base.extra_data(extra_data);
        self
    }

    pub fn system(mut self, info: Value) -> Self {
        self.base = self.base.system(info);
        self
    }

    pub fn system_id(mut self, system_id: SystemId) -> Self {
        self.base = self.base.system_id(system_id);
        self
    }

    pub fn inputs(mut self, inputs: Vec<u8>) -> Self {
        self.base = self.base.inputs(inputs);
        self
    }

    /// return the builder with added reward/stake parameters
    pub fn set_token_params(
        mut self,
        minimum_stake: u128,
        minimum_reward_amount: U256,
        maximum_reward_amount: U256,
    ) -> Self {
        self.minimum_stake = minimum_stake;
        self.min_reward_amount = minimum_reward_amount;
        self.max_reward_amount = maximum_reward_amount;
        self
    }

    pub fn max_reward_amount(mut self, reward_amount: U256) -> Self {
        self.max_reward_amount = reward_amount;
        self
    }

    pub fn min_reward_amount(mut self, reward_amount: U256) -> Self {
        self.min_reward_amount = reward_amount;
        self
    }

    pub fn minimum_stake(mut self, stake_amount: u128) -> Self {
        self.minimum_stake = stake_amount;
        self
    }
}

impl<T, P, N> IntentBuilder for ComputeRequestBuilder<T, P, N>
where
    T: Transport + Clone + Send + Sync,
    P: Provider<T, N> + Clone + Send + Sync,
    N: Network + Clone + Send + Sync,
{
    type Intent = ComputeRequest<SystemParams>;

    /// return the Intent derived from the current state of Builder
    fn build(&self) -> Result<ComputeRequest<SystemParams>> {
        let system = self.base.build_system()?;
        Ok(ComputeRequest {
            system_id: self.base.system_id,
            system,
            proof_request: ProofRequest {
                signer: self.base.signer_address,
                market: self.base.market_address,
                nonce: self.base.nonce,
                rewardToken: self.base.reward_token_address,
                maxRewardAmount: self.max_reward_amount,
                minRewardAmount: self.min_reward_amount,
                minimumStake: self.minimum_stake,
                startAuctionTimestamp: self.base.start_auction_timestamp,
                endAuctionTimestamp: self.base.end_auction_timestamp,
                provingTime: self.base.proving_time,
                inputsCommitment: self.base.inputs_commitment,
                extraData: self.base.extra_data.clone(),
            },
            signature: BaseIntentBuilder::<T, P, N>::create_dummy_signature(),
        })
    }
}
