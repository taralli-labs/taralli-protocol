use std::time::Duration;

use alloy::primitives::Address;
use alloy::signers::Signer;
use alloy::{network::Network, providers::Provider, transports::Transport};
use taralli_primitives::intents::request::ComputeRequest;
use taralli_primitives::intents::ComputeIntent;
use taralli_primitives::systems::{SystemId, SystemParams};
use taralli_primitives::validation::request::{
    ComputeRequestValidator, RequestValidationConfig, RequestVerifierConstraints,
};
use taralli_primitives::validation::IntentValidator;
use url::Url;

use crate::api::submit::SubmitApiClient;
use crate::error::{ClientError, Result};
use crate::tracker::{IntentAuctionTracker, IntentResolveTracker};
use crate::{
    intent_builder::request::ComputeRequestBuilder, tracker::request::ComputeRequestTracker,
};

use crate::client::BaseClient;

/// Client that submits signed ComputeRequest to the protocol server, tracks their auction status
/// and then tracks their resolution status to see if the requested compute workload was fulfilled.
pub struct RequesterRequestingClient<T, P, N, S>
where
    T: Transport + Clone,
    P: Provider<T, N> + Clone,
    N: Network + Clone,
{
    pub base: BaseClient<T, P, N, S>,
    pub api: SubmitApiClient,
    pub validator: ComputeRequestValidator,
    pub builder: ComputeRequestBuilder<T, P, N>,
    pub tracker: ComputeRequestTracker<T, P, N>,
}

impl<T, P, N, S> RequesterRequestingClient<T, P, N, S>
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
        validation_config: RequestValidationConfig,
        verifier_constraints: RequestVerifierConstraints,
    ) -> Self {
        Self {
            base: BaseClient::new(rpc_provider.clone(), signer.clone(), market_address),
            api: SubmitApiClient::new(server_url),
            validator: ComputeRequestValidator::new(validation_config, verifier_constraints),
            builder: ComputeRequestBuilder::new(
                rpc_provider.clone(),
                signer.address(),
                market_address,
                system_id,
            ),
            tracker: ComputeRequestTracker::new(rpc_provider, market_address),
        }
    }

    /// sign the inputted proof request and submit it to the taralli server.
    /// then start tracking the request auction and resolution on-chain.
    pub async fn submit_and_track(
        &self,
        request: ComputeRequest<SystemParams>,
        auction_time_length: u64,
    ) -> Result<()> {
        // compute request id
        let request_id = request.compute_id();

        // compute resolve deadline timestamp
        let resolve_deadline =
            request.proof_request.endAuctionTimestamp + request.proof_request.provingTime as u64;

        // setup tracking
        let auction_tracker = self
            .tracker
            .track_auction(request_id, Duration::from_secs(auction_time_length));
        let resolution_tracker = self
            .tracker
            .track_resolve(request_id, Duration::from_secs(resolve_deadline));

        tracing::info!("tracking setup for request ID: {}", request_id);
        tracing::info!("submitting request to server");

        // submit signed request to server
        let response = self
            .api
            .submit_intent(request)
            .await
            .map_err(|e| ClientError::ServerRequestError(e.to_string()))?;

        if !response.status().is_success() {
            // Get the response text instead of trying to parse JSON directly
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            // Try to parse as JSON if possible
            let error_message = match serde_json::from_str::<serde_json::Value>(&error_text) {
                Ok(json) => {
                    if let Some(error) = json.get("error").and_then(|e| e.as_str()) {
                        format!("Server validation failed: {}", error)
                    } else {
                        format!("Server returned error: {}", error_text)
                    }
                }
                Err(_) => format!("Server returned error: {}", error_text),
            };

            return Err(ClientError::IntentSubmissionFailed(error_message));
        }

        tracing::info!("Request submitted successfully, waiting for auction result");

        // Wait for auction result
        let _auction_result = auction_tracker
            .await
            .map_err(|e| ClientError::TrackIntentError(e.to_string()))?
            .ok_or(ClientError::AuctionTimeoutError())?;

        tracing::info!("Auction completed, waiting for resolution");

        // Wait for resolution
        let _resolution_result = resolution_tracker
            .await
            .map_err(|e| ClientError::TrackIntentError(e.to_string()))?;

        tracing::info!("Tracking complete");
        Ok(())
    }

    pub async fn sign(
        &self,
        mut request: ComputeRequest<SystemParams>,
    ) -> Result<ComputeRequest<SystemParams>> {
        // build permit2 digest
        let permit2_digest = request.compute_permit2_digest();

        // sign permit2 digest
        let signature = self
            .base
            .signer
            .sign_hash(&permit2_digest)
            .await
            .map_err(|e| ClientError::IntentSigningError(e.to_string()))?;
        // load signature into proof request
        request.signature = signature;
        Ok(request)
    }

    pub fn validate_request(&self, request: &ComputeRequest<SystemParams>) -> Result<()> {
        // validate a request built by the requester client
        self.validator.validate(
            request,
            request.proof_request.startAuctionTimestamp,
            &request.proof_request.market,
        )?;
        Ok(())
    }
}
