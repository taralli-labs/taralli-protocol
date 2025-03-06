use crate::error::{ClientError, Result};
use alloy::network::ReceiptResponse;
use alloy::primitives::FixedBytes;
use async_trait::async_trait;
use std::marker::PhantomData;
use taralli_primitives::abi::universal_bombetta::UniversalBombetta::{
    ProofRequest, UniversalBombettaInstance,
};
use taralli_primitives::alloy::network::Network;
use taralli_primitives::alloy::primitives::{Address, Bytes, PrimitiveSignature, U256};
use taralli_primitives::alloy::providers::Provider;
use taralli_primitives::alloy::transports::Transport;
use tokio::time::{sleep, Duration};

use super::IntentBidder;

#[derive(Clone)]
pub struct ComputeRequestBidder<T, P, N> {
    rpc_provider: P,
    market_address: Address,
    phantom_data: PhantomData<(T, N)>,
}

#[derive(Clone)]
pub struct ComputeRequestBidParams {
    pub target_amount: U256,
}

impl<T, P, N> ComputeRequestBidder<T, P, N>
where
    T: Transport + Clone,
    P: Provider<T, N> + Clone,
    N: Network + Clone,
{
    pub fn new(rpc_provider: P, market_address: Address) -> Self {
        Self {
            rpc_provider,
            market_address,
            phantom_data: PhantomData,
        }
    }
}

#[async_trait]
impl<T, P, N> IntentBidder<N> for ComputeRequestBidder<T, P, N>
where
    T: Transport + Clone,
    P: Provider<T, N> + Clone,
    N: Network + Clone,
{
    type IntentProofCommitment = ProofRequest;
    type BidParameters = ComputeRequestBidParams;

    async fn submit_bid(
        &self,
        latest_ts: u64,
        intent_id: FixedBytes<32>,
        bid_params: Self::BidParameters,
        intent_proof_commitment: Self::IntentProofCommitment,
        signature: PrimitiveSignature,
    ) -> Result<N::ReceiptResponse> {
        let market_contract =
            UniversalBombettaInstance::new(self.market_address, self.rpc_provider.clone());

        // check auction has started
        if latest_ts < intent_proof_commitment.startAuctionTimestamp {
            return Err(ClientError::TransactionSetupError(
                "Auction has not started based on current block ts".into(),
            ));
        }

        // check that the deadline is not passed
        if latest_ts > intent_proof_commitment.endAuctionTimestamp {
            return Err(ClientError::TransactionSetupError(
                "Auction has expired".into(),
            ));
        }

        tracing::info!("bidder: check timestamps done");

        // auction is active, calculate target timestamp from target_amount
        let current_estimated_amount = calculate_current_reward(
            latest_ts,
            intent_proof_commitment.startAuctionTimestamp,
            intent_proof_commitment.endAuctionTimestamp,
            intent_proof_commitment.minRewardAmount,
            intent_proof_commitment.maxRewardAmount,
        );

        tracing::info!(
            "bidder: current_estimated_amount: {}",
            current_estimated_amount
        );

        if current_estimated_amount < bid_params.target_amount {
            // wait ideal number of seconds to get +/- the target_amount, then send bid
            let target_timestamp = calculate_target_timestamp(
                bid_params.target_amount,
                intent_proof_commitment.startAuctionTimestamp,
                intent_proof_commitment.endAuctionTimestamp,
                intent_proof_commitment.minRewardAmount,
                intent_proof_commitment.maxRewardAmount,
            )?;
            let wait_time = target_timestamp - latest_ts;
            tracing::info!("bidder: waiting {} seconds for ideal amount", wait_time);
            // Wait for `wait_time` seconds
            sleep(Duration::from_secs(wait_time)).await;
        }

        tracing::info!("bidder: calculate target ts for target amount");

        tracing::info!("bidder: latest_ts: {:?}", latest_ts);
        tracing::info!("bidder: request id computed: {}", intent_id);

        let active_request_return = market_contract
            .activeProofRequestData(intent_id)
            .call()
            .await
            .map_err(|e| ClientError::TransactionSetupError(e.to_string()))?;

        if active_request_return.requester != Address::ZERO {
            return Err(ClientError::TransactionSetupError(
                "Another Bid has already submitted for this Auction".into(),
            ));
        }

        let gas_estimate = market_contract
            .bid(
                intent_proof_commitment.clone(),
                Bytes::from(signature.as_bytes()),
            )
            .value(U256::from(intent_proof_commitment.minimumStake))
            .estimate_gas()
            .await
            .map_err(|e| {
                ClientError::TransactionSetupError(format!("Gas estimation failed: {}", e))
            })?;

        tracing::info!("Estimated gas for bid: {}", gas_estimate);
        tracing::info!(
            "msg.value for bid: {}",
            U256::from(intent_proof_commitment.minimumStake)
        );

        let receipt = market_contract
            .bid(
                intent_proof_commitment.clone(),
                Bytes::from(signature.as_bytes()),
            )
            .value(U256::from(intent_proof_commitment.minimumStake))
            .send()
            .await
            .map_err(|e| ClientError::TransactionError(e.to_string()))?
            .get_receipt()
            .await
            .map_err(|e| ClientError::TransactionFailure(e.to_string()))?;

        tracing::info!("bid txs receipt: {:?}", receipt);

        // Check if the transaction was reverted
        if !receipt.status() {
            return Err(ClientError::TransactionFailure(
                "Transaction reverted on-chain".into(),
            ));
        }

        Ok(receipt)
    }
}

fn calculate_current_reward(
    current_timestamp: u64,
    start_timestamp: u64,
    end_timestamp: u64,
    min_reward: U256,
    max_reward: U256,
) -> U256 {
    if current_timestamp == start_timestamp {
        return max_reward; // At auction start, return maxRewardAmount
    }
    if current_timestamp == end_timestamp {
        return min_reward; // At auction end, return minRewardAmount
    }

    // calculate factor to decrease by to get estimated current amount
    let elapsed_time = U256::from(current_timestamp - start_timestamp);
    let total_duration = U256::from(end_timestamp - start_timestamp);
    let reward_range = max_reward - min_reward;
    let decrease_amount = (elapsed_time * reward_range) / total_duration;

    max_reward - decrease_amount
}

fn calculate_target_timestamp(
    target_amount: U256,
    start_timestamp: u64,
    end_timestamp: u64,
    min_reward: U256,
    max_reward: U256,
) -> Result<u64> {
    // Ensure target_amount is within min_reward and max_reward
    if target_amount < min_reward || target_amount > max_reward {
        return Err(ClientError::TransactionSetupError(
            "Target amount is out of bounds".into(),
        ));
    }

    let total_duration = U256::from(end_timestamp - start_timestamp);
    let elapsed_time = total_duration * (target_amount - min_reward) / (max_reward - min_reward);
    let target_timestamp = U256::from(start_timestamp) + elapsed_time;

    // Convert U256 back to u64
    target_timestamp.try_into().map_err(|e| {
        ClientError::TransactionSetupError(format!("Failed to convert target timestamp: {}", e))
    })
}
