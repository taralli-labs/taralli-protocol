use crate::error::{ClientError, Result};
use async_trait::async_trait;
use std::marker::PhantomData;
use taralli_primitives::abi::universal_bombetta::UniversalBombetta::{
    ProofRequest, UniversalBombettaInstance,
};
use taralli_primitives::alloy::network::Network;
use taralli_primitives::alloy::primitives::{Address, Bytes, PrimitiveSignature, U256};
use taralli_primitives::alloy::providers::Provider;
use taralli_primitives::alloy::transports::Transport;
use taralli_primitives::utils::compute_request_id;
use tokio::time::{sleep, Duration};

use super::IntentBidder;

#[derive(Clone)]
pub struct ComputeRequestBidder<T, P, N> {
    rpc_provider: P,
    market_address: Address,
    // min_bid_delay: u64,
    // _max_bid_attempts: u32,
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

        // auction is active, calculate target timestamp from target_amount (amount of
        // reward tokens)
        let current_estimated_amount = calculate_current_reward(
            latest_ts,
            intent_proof_commitment.startAuctionTimestamp,
            intent_proof_commitment.endAuctionTimestamp,
            intent_proof_commitment.minRewardAmount,
            intent_proof_commitment.maxRewardAmount,
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
            // Wait for `wait_time` seconds
            sleep(Duration::from_secs(wait_time)).await;
        }

        tracing::info!("bidder: calculate target ts for target amount");

        // check the proof request does not already have a bid
        let request_id = compute_request_id(&intent_proof_commitment, &signature);

        let active_request_return = market_contract
            .activeProofRequestData(request_id)
            .call()
            .await
            .map_err(|e| ClientError::TransactionSetupError(e.to_string()))?;

        if active_request_return.requester != Address::ZERO {
            return Err(ClientError::TransactionSetupError(
                "Another Bid has already submitted for this Auction".into(),
            ));
        }

        tracing::info!("bidder: check status of auction again to make sure no bid is submitted");
        tracing::info!(
            "bidder: requester address = {}",
            active_request_return.requester
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

        Ok(receipt)
    }
}

/// Helper methods

fn calculate_current_reward(
    current_timestamp: u64,
    start_timestamp: u64,
    end_timestamp: u64,
    min_reward: U256,
    max_reward: U256,
) -> U256 {
    let elapsed_time = U256::from(current_timestamp - start_timestamp);
    let total_duration = U256::from(end_timestamp - start_timestamp);
    // increase factor
    let increase_factor = elapsed_time * U256::from(1e18) / total_duration;
    // Calculate the increased amount
    let increase_amount = increase_factor * (max_reward - min_reward) / U256::from(1e18);
    // calculate current reward amount
    min_reward + increase_amount
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
