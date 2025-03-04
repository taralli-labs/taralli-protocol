use alloy::primitives::Address;
use alloy::signers::Signer;
use alloy::{network::Network, providers::Provider, transports::Transport};
use std::sync::Arc;
use std::time::Duration;
use taralli_primitives::intents::offer::ComputeOffer;
use taralli_primitives::systems::{SystemId, SystemParams};
use taralli_primitives::utils::{
    compute_offer_id, compute_offer_permit2_digest, compute_offer_witness,
};
use taralli_primitives::validation::offer::OfferValidationConfig;
use taralli_primitives::validation::Validate;
use url::Url;

use crate::api::submit::SubmitApiClient;
use crate::client::BaseClient;
use crate::error::{ClientError, Result};
use crate::resolver::IntentResolver;
use crate::tracker::IntentAuctionTracker;
use crate::worker::{ComputeWorker, WorkResult};
use crate::{
    intent_builder::offer::ComputeOfferBuilder, resolver::offer::ComputeOfferResolver,
    tracker::ComputeOfferTracker,
};

pub struct ProviderOfferingClient<T, P, N, S>
where
    T: Transport + Clone,
    P: Provider<T, N> + Clone,
    N: Network + Clone,
{
    base: BaseClient<T, P, N, S>,
    api: SubmitApiClient,
    builder: ComputeOfferBuilder<T, P, N>,
    tracker: ComputeOfferTracker<T, P, N>,
    worker: Arc<dyn ComputeWorker<ComputeOffer<SystemParams>>>,
    resolver: ComputeOfferResolver<T, P, N>,
}

impl<T, P, N, S> ProviderOfferingClient<T, P, N, S>
where
    T: Transport + Clone,
    P: Provider<T, N> + Clone,
    N: Network + Clone,
    S: Signer + Clone,
{
    pub fn new(
        server_url: Url,
        rpc_provider: P,
        signer: S,
        market_address: Address,
        system_id: SystemId,
        worker: Arc<dyn ComputeWorker<ComputeOffer<SystemParams>>>,
        validation_config: OfferValidationConfig,
    ) -> Self {
        Self {
            base: BaseClient::new(rpc_provider.clone(), signer.clone(), market_address),
            api: SubmitApiClient::new(server_url.clone()),
            builder: ComputeOfferBuilder::new(
                rpc_provider.clone(),
                signer.address(),
                market_address,
                system_id,
                validation_config,
            ),
            tracker: ComputeOfferTracker::new(rpc_provider.clone(), market_address),
            worker,
            resolver: ComputeOfferResolver::new(rpc_provider, market_address),
        }
    }

    /// sign the inputted proof offer and submit it to the taralli server.
    /// then start tracking the offer auction on-chain.
    pub async fn submit_and_track(
        &self,
        offer: ComputeOffer<SystemParams>,
        auction_time_length: u64,
    ) -> Result<()> {
        // compute id
        let offer_id = compute_offer_id(&offer.proof_offer, &offer.signature);

        // compute resolve deadline timestamp
        let _resolve_deadline =
            offer.proof_offer.endAuctionTimestamp + offer.proof_offer.provingTime as u64;

        // setup tracking
        let auction_tracker = self
            .tracker
            .track_auction(offer_id, Duration::from_secs(auction_time_length));

        tracing::info!("tracking started for offer ID: {}", offer_id);
        tracing::info!("submitting offer to server");

        // submit signed request to server
        let response = self
            .api
            .submit_intent(offer.clone())
            .await
            .map_err(|e| ClientError::ServerRequestError(e.to_string()))?;

        // track the offer's auction
        if !response.status().is_success() {
            // Parse the error response
            let error_body = response.json::<serde_json::Value>().await.map_err(|e| {
                ClientError::ServerRequestError(format!("Failed to parse error response: {}", e))
            })?;

            return Err(ClientError::IntentSubmissionFailed(format!(
                "Server validation failed: {}",
                error_body["error"].as_str().unwrap_or("Unknown error")
            )));
        }

        tracing::info!("Request submitted successfully, waiting for auction result");

        // Wait for auction result
        let _auction_result = auction_tracker
            .await
            .map_err(|e| ClientError::TrackIntentError(e.to_string()))?
            .ok_or(ClientError::AuctionTimeoutError())?;

        tracing::info!("Auction completed, starting compute worker");

        // Execute worker
        let work_result: WorkResult = self
            .worker
            .execute(&offer)
            .await
            .map_err(|e| ClientError::WorkerError(e.to_string()))?;

        tracing::info!("Compute worker execution completed, resolving");

        self.resolver
            .resolve_intent(offer_id, work_result.opaque_submission)
            .await
            .map_err(|e| ClientError::TransactionFailure(format!("resolver failed: {}", e)))?;

        tracing::info!("Compute offer resolved");
        Ok(())
    }

    pub async fn sign(
        &self,
        mut offer: ComputeOffer<SystemParams>,
    ) -> Result<ComputeOffer<SystemParams>> {
        // compute witness
        let witness = compute_offer_witness(&offer.proof_offer);
        // build permit2 digest
        let permit2_digest = compute_offer_permit2_digest(&offer.proof_offer, witness);
        // sign permit2 digest
        let signature = self
            .base
            .signer
            .sign_hash(&permit2_digest)
            .await
            .map_err(|e| ClientError::IntentSigningError(e.to_string()))?;
        // load signature into proof request
        offer.signature = signature;
        Ok(offer)
    }

    pub fn validate_offer(&self, offer: &ComputeOffer<SystemParams>) -> Result<()> {
        // validate an offer built by the client
        offer.validate(
            offer.proof_offer.startAuctionTimestamp,
            &self.base.market_address,
            &self.builder.validation_config,
        )?;
        Ok(())
    }
}
