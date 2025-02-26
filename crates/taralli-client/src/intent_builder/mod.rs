pub mod offer;
pub mod request;

use serde_json::Value;
use taralli_primitives::alloy::primitives::{Address, Bytes, PrimitiveSignature, B256, U256};
use taralli_primitives::alloy::{
    consensus::BlockHeader,
    eips::BlockId,
    network::{BlockResponse, BlockTransactionsKind, Network},
    providers::Provider,
    transports::Transport,
};
use taralli_primitives::systems::SystemId;
use taralli_primitives::systems::SystemParams;

use crate::{
    error::{ClientError, Result},
    nonce_manager::Permit2NonceManager,
};

/// core builder trait
pub trait IntentBuilder {
    type Intent;
    fn build(&self) -> Result<Self::Intent>;
}

/// signature bytes used as placeholder before signing
pub const MOCK_SIGNATURE_BYTES: [u8; 65] = [
    132, 12, 252, 87, 40, 69, 245, 120, 110, 112, 41, 132, 194, 165, 130, 82, 140, 173, 75, 73,
    178, 161, 11, 157, 177, 190, 127, 202, 144, 5, 133, 101, 37, 231, 16, 156, 235, 152, 22, 141,
    149, 176, 155, 24, 187, 246, 182, 133, 19, 14, 5, 98, 242, 51, 135, 125, 73, 43, 148, 238, 224,
    197, 182, 209, 0,
];

#[derive(Clone)]
pub struct BaseIntentBuilder<T, P, N>
where
    T: Transport + Clone + Send + Sync,
    P: Provider<T, N> + Clone + Send + Sync,
    N: Network + Clone + Send + Sync,
{
    rpc_provider: P,
    permit2_nonce_manager: Permit2NonceManager<T, P, N>,
    signer_address: Address,
    auction_length: u32,
    // general proof commitment params
    pub market_address: Address,
    pub nonce: U256,
    pub reward_token_address: Address,
    pub reward_token_decimals: u8,
    pub start_auction_timestamp: u64,
    pub end_auction_timestamp: u64,
    pub proving_time: u32,
    pub inputs_commitment: B256,
    pub extra_data: Bytes,
    // general system params
    system_id: SystemId,
    system: serde_json::Value,
    pub inputs: Vec<u8>,
}

impl<T, P, N> BaseIntentBuilder<T, P, N>
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

        Self {
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
        }
    }

    /// return the RequestBuilder with the added permit2 nonce
    pub async fn set_new_nonce(mut self) -> Result<Self> {
        self.nonce = self
            .permit2_nonce_manager
            .get_nonce()
            .await
            .map_err(|e| ClientError::GetNonceError(e.to_string()))?;
        Ok(self)
    }

    /// return the RequestBuilder with the added auction timestamps based on auction length
    /// and the current latest block timestamp
    pub async fn set_auction_timestamps_from_auction_length(mut self) -> Result<Self> {
        if self.auction_length == 0 {
            return Err(ClientError::SetAuctionTimestampsError());
        }
        let (latest_ts, computed_end_ts) = self
            .calculate_timestamp_params_from_current_timestamp(self.auction_length)
            .await?;
        self.start_auction_timestamp = latest_ts;
        self.end_auction_timestamp = computed_end_ts;
        Ok(self)
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
            .map_err(|e| ClientError::RpcRequestError(e.to_string()))?
            .ok_or_else(|| ClientError::RpcRequestError("Latest block not found".to_string()))?;

        let start_auction_timestamp = latest_block.header().timestamp();
        let end_auction_timestamp = start_auction_timestamp + auction_length as u64;

        Ok((start_auction_timestamp, end_auction_timestamp))
    }

    /// return the IntentBuilder with the added auction time parameters
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
        inputs_commitment: B256,
        extra_data: Bytes,
    ) -> Self {
        self.inputs_commitment = inputs_commitment;
        self.extra_data = extra_data;
        self
    }

    pub fn build_system(&self) -> Result<SystemParams> {
        SystemParams::try_from((&self.system_id, self.system.to_string().into_bytes()))
            .map_err(|e| ClientError::BuilderError(e.to_string()))
    }

    /// create dummy ECDSA signature
    pub fn create_dummy_signature() -> PrimitiveSignature {
        PrimitiveSignature::try_from(&MOCK_SIGNATURE_BYTES[..])
            .expect("Unreachable: Mock Signature try from failure")
    }

    pub fn auction_length(mut self, auction_length: u32) -> Self {
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

    pub fn reward_token_decimals(mut self, token_decimals: u8) -> Self {
        self.reward_token_decimals = token_decimals;
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

    pub fn extra_data(mut self, extra_data: Bytes) -> Self {
        self.extra_data = extra_data;
        self
    }

    pub fn system(mut self, info: Value) -> Self {
        self.system = info;
        self
    }

    pub fn system_id(mut self, system_id: SystemId) -> Self {
        self.system_id = system_id;
        self
    }

    pub fn inputs(mut self, inputs: Vec<u8>) -> Self {
        self.inputs = inputs;
        self
    }
}
