use crate::error::{ClientError, Result};
use alloy::primitives::FixedBytes;
use async_trait::async_trait;
use std::marker::PhantomData;
use taralli_primitives::abi::universal_porchetta::UniversalPorchetta::{
    ProofOffer, UniversalPorchettaInstance,
};
use taralli_primitives::alloy::network::Network;
use taralli_primitives::alloy::primitives::{Address, Bytes, PrimitiveSignature};
use taralli_primitives::alloy::providers::Provider;
use taralli_primitives::alloy::transports::Transport;

use super::IntentBidder;

/// Bid on a ComputeOffer
#[derive(Clone)]
pub struct ComputeOfferBidder<T, P, N> {
    rpc_provider: P,
    market_address: Address,
    phantom_data: PhantomData<(T, N)>,
}

pub struct ComputeOfferBidParams {}

impl<T, P, N> ComputeOfferBidder<T, P, N>
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
impl<T, P, N> IntentBidder<N> for ComputeOfferBidder<T, P, N>
where
    T: Transport + Clone,
    P: Provider<T, N> + Clone,
    N: Network + Clone,
{
    type IntentProofCommitment = ProofOffer;
    type BidParameters = ComputeOfferBidParams;

    async fn submit_bid(
        &self,
        latest_ts: u64,
        intent_id: FixedBytes<32>,
        _bid_params: Self::BidParameters,
        intent_proof_commitment: Self::IntentProofCommitment,
        signature: PrimitiveSignature,
    ) -> Result<N::ReceiptResponse> {
        let market_contract =
            UniversalPorchettaInstance::new(self.market_address, self.rpc_provider.clone());

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

        let active_offer_return = market_contract
            .activeProofOfferData(intent_id)
            .call()
            .await
            .map_err(|e| ClientError::TransactionSetupError(e.to_string()))?;

        if active_offer_return.provider != Address::ZERO {
            return Err(ClientError::TransactionSetupError(
                "Another Bid has already submitted for this Auction".into(),
            ));
        }

        tracing::info!(
            "bidder: requester address = {}",
            active_offer_return.requester
        );

        let receipt = market_contract
            .bid(
                intent_proof_commitment.clone(),
                Bytes::from(signature.as_bytes()),
            )
            .send()
            .await
            .map_err(|e| ClientError::TransactionError(e.to_string()))?
            .get_receipt()
            .await
            .map_err(|e| ClientError::TransactionFailure(e.to_string()))?;

        Ok(receipt)
    }
}
