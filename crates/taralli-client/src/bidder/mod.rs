use crate::error::Result;
use alloy::primitives::FixedBytes;
use async_trait::async_trait;
use taralli_primitives::alloy::network::Network;
use taralli_primitives::alloy::primitives::PrimitiveSignature;

pub mod offer;
pub mod request;

/// core bidder trait
#[async_trait]
pub trait IntentBidder<N: Network> {
    type IntentProofCommitment;
    type BidParameters;
    async fn submit_bid(
        &self,
        latest_ts: u64,
        intent_id: FixedBytes<32>,
        bid_params: Self::BidParameters,
        proof_commitment: Self::IntentProofCommitment,
        signature: PrimitiveSignature,
    ) -> Result<N::ReceiptResponse>;
}
