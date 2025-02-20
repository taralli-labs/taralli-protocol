use serde_json::Value;
use taralli_primitives::abi::universal_bombetta::UniversalBombetta::ProofRequest;
use taralli_primitives::alloy::{
    network::Network,
    primitives::U256,
    primitives::{Address, Bytes, B256},
    providers::Provider,
    transports::Transport,
};
use taralli_primitives::intents::ComputeRequest;
use taralli_primitives::systems::{SystemId, SystemParams};

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
        }
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
