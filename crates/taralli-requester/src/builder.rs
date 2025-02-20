use crate::config::RewardTokenConfig;
use crate::create_dummy_signature;
use crate::error::{RequesterError, Result};
use crate::nonce_manager::Permit2NonceManager;
use serde_json::Value;
use taralli_primitives::alloy::{
    consensus::BlockHeader,
    eips::BlockId,
    network::{BlockResponse, BlockTransactionsKind, Network},
    primitives::{Address, Bytes, B256, U256},
    providers::Provider,
    transports::Transport,
};
use taralli_primitives::systems::{ProvingSystemId, ProvingSystemParams};
use taralli_primitives::{OnChainProofRequest, Request};

// TODO: add in default builder patterns
pub struct AuctionParameters<P: Into<U256>> {
    pub auction_len: u32,
    pub floor_price: P,
    pub ceiling_price: P,
    pub reward_token_config: RewardTokenConfig,
}

#[derive(Clone)]
pub struct RequestBuilder<T, P, N> {
    // rpc provider
    rpc_provider: P,
    // permit2 nonce manager
    permit2_nonce_manager: Permit2NonceManager<T, P, N>,
    // signer address
    signer_address: Address,
    // auction length
    auction_length: u32,
    // on-chain params
    pub market_address: Address,
    pub nonce: U256,
    pub reward_token_address: Address,
    pub max_reward_amount: U256,
    pub min_reward_amount: U256,
    pub minimum_stake: u128,
    pub start_auction_timestamp: u64,
    pub end_auction_timestamp: u64,
    pub proving_time: u32,
    pub public_inputs_commitment: B256,
    pub extra_data: Bytes,
    // off-chain params
    pub proving_system_id: ProvingSystemId,
    pub proving_system_information: serde_json::Value,
    pub public_inputs: Vec<u8>,
}

impl<T, P, N> RequestBuilder<T, P, N>
where
    T: Transport + Clone,
    P: Provider<T, N> + Clone,
    N: Network + Clone,
{
    pub fn new(
        rpc_provider: P,
        signer_address: Address,
        market_address: Address,
        proving_system_id: ProvingSystemId,
    ) -> Self {
        // build permit2 nonce manager
        let permit2_nonce_manager = Permit2NonceManager::new(rpc_provider.clone(), signer_address);

        Self {
            rpc_provider,
            permit2_nonce_manager,
            signer_address,
            market_address,
            auction_length: 0u32,
            nonce: U256::ZERO,
            reward_token_address: Address::ZERO,
            max_reward_amount: U256::ZERO,
            min_reward_amount: U256::ZERO,
            minimum_stake: 0u128,
            start_auction_timestamp: 0u64,
            end_auction_timestamp: 0u64,
            proving_time: 0u32,
            public_inputs_commitment: B256::ZERO,
            extra_data: Bytes::from(""),
            proving_system_id,
            proving_system_information: Value::Null,
            public_inputs: vec![],
        }
    }

    /// return the RequestBuilder with the added permit2 nonce
    pub async fn set_new_nonce(mut self) -> Result<Self> {
        self.nonce = self
            .permit2_nonce_manager
            .get_nonce()
            .await
            .map_err(|e| RequesterError::GetNonceError(e.to_string()))?;
        Ok(self)
    }

    /// return the RequestBuilder with the added auction timestamps based on auction length
    /// and the current latest block timestamp
    pub async fn set_auction_timestamps_from_auction_length(mut self) -> Result<Self> {
        if self.auction_length == 0 {
            return Err(RequesterError::SetAuctionTimestampsError());
        }
        let (latest_ts, computed_end_ts) = self
            .calculate_timestamp_params_from_current_timestamp(self.auction_length)
            .await?;
        self.start_auction_timestamp = latest_ts;
        self.end_auction_timestamp = computed_end_ts;
        Ok(self)
    }

    /// return the RequestBuilder with added reward/stake parameters
    pub fn set_reward_params(
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

    /// return the RequestBuilder with the added auction time parameters
    pub fn set_time_params(
        mut self,
        start_auction_ts: u64,
        end_auction_ts: u64,
        proving_time: u32,
    ) -> Self {
        self.start_auction_timestamp = start_auction_ts;
        self.end_auction_timestamp = end_auction_ts;
        self.proving_time = proving_time;
        self
    }

    /// return the ProofRequest builder with the added verification commitments
    pub fn set_verification_commitment_params(
        mut self,
        public_inputs_commitment: B256,
        extra_data: Bytes,
    ) -> Self {
        self.public_inputs_commitment = public_inputs_commitment;
        self.extra_data = extra_data;
        self
    }

    /// return the ProofRequest derived from the current state of RequestBuilder
    pub fn build(self) -> Request<ProvingSystemParams> {
        Request {
            proving_system_id: self.proving_system_id,
            proving_system_information: ProvingSystemParams::try_from((
                &self.proving_system_id,
                self.proving_system_information.to_string().into_bytes(),
            ))
            .unwrap(),
            onchain_proof_request: OnChainProofRequest {
                signer: self.signer_address,
                market: self.market_address,
                nonce: self.nonce,
                token: self.reward_token_address,
                maxRewardAmount: self.max_reward_amount,
                minRewardAmount: self.min_reward_amount,
                minimumStake: self.minimum_stake,
                startAuctionTimestamp: self.start_auction_timestamp,
                endAuctionTimestamp: self.end_auction_timestamp,
                provingTime: self.proving_time,
                publicInputsCommitment: self.public_inputs_commitment,
                extraData: self.extra_data,
            },
            signature: create_dummy_signature(),
        }
    }

    /// fetch latest timestamp and return (start auction timestamp, end auction timestamp) based
    /// on the inputted auction length using the latest timestamp
    async fn calculate_timestamp_params_from_current_timestamp(
        &self,
        auction_length: u32,
    ) -> Result<(u64, u64)> {
        let latest_block = self
            .rpc_provider
            .get_block(BlockId::latest(), BlockTransactionsKind::Hashes)
            .await
            .map_err(|e| RequesterError::RpcRequestError(e.to_string()))?
            .ok_or_else(|| RequesterError::RpcRequestError("Latest block not found".to_string()))?;

        let start_auction_timestamp = latest_block.header().timestamp();
        let end_auction_timestamp = start_auction_timestamp + auction_length as u64;

        Ok((start_auction_timestamp, end_auction_timestamp))
    }

    pub fn set_auction_length(mut self, auction_length: u32) -> Self {
        self.auction_length = auction_length;
        self
    }

    pub fn market_address(mut self, market_address: Address) -> Self {
        self.market_address = market_address;
        self
    }

    pub fn nonce(mut self, nonce: U256) -> Self {
        self.nonce = nonce;
        self
    }

    pub fn reward_token_address(mut self, token_address: Address) -> Self {
        self.reward_token_address = token_address;
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

    pub fn start_auction_timestamp(mut self, timestamp: u64) -> Self {
        self.start_auction_timestamp = timestamp;
        self
    }

    pub fn end_auction_timestamp(mut self, timestamp: u64) -> Self {
        self.end_auction_timestamp = timestamp;
        self
    }

    pub fn proving_time(mut self, seconds_to_prove: u32) -> Self {
        self.proving_time = seconds_to_prove;
        self
    }

    pub fn proving_system_information(mut self, info: Value) -> Self {
        self.proving_system_information = info;
        self
    }

    pub fn extra_data(mut self, extra_data: Bytes) -> Self {
        self.extra_data = extra_data;
        self
    }

    pub fn proving_system_id(mut self, proving_system_id: ProvingSystemId) -> Self {
        self.proving_system_id = proving_system_id;
        self
    }

    pub fn public_inputs(mut self, public_inputs: Vec<u8>) -> Self {
        self.public_inputs = public_inputs;
        self
    }
}
