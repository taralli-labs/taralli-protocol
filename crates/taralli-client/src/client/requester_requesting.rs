use std::time::Duration;

use alloy::primitives::Address;
use alloy::signers::Signer;
use alloy::{network::Network, providers::Provider, transports::Transport};
use taralli_primitives::intents::ComputeRequest;
use taralli_primitives::systems::{SystemId, SystemParams};
use taralli_primitives::utils::{compute_request_id, compute_request_permit2_digest, compute_request_witness};
use url::Url;
//use taralli_primitives::validation::Validate;

use crate::tracker::{IntentAuctionTracker, IntentResolveTracker};
use crate::{intent_builder::request::ComputeRequestBuilder, tracker::ComputeRequestTracker};
use crate::error::{Result, ClientError};

use super::BaseClient;

pub struct RequesterRequestingClient<T, P, N, S>
where
    T: Transport + Clone,
    P: Provider<T, N> + Clone,
    N: Network + Clone,
{
    base: BaseClient<T, P, N, S>,
    builder: ComputeRequestBuilder<T, P, N>,
    tracker: ComputeRequestTracker<T, P, N>,
}

impl<T, P, N, S> RequesterRequestingClient<T, P, N, S>
where
    T: Transport + Clone,
    P: Provider<T, N> + Clone,
    N: Network + Clone,
    S: Signer + Clone
{
    pub fn new(
        server_url: Url,
        rpc_provider: P,
        signer: S,
        market_address: Address,
        system_id: SystemId
    ) -> Self {
        Self {
            base: BaseClient::new(server_url, rpc_provider.clone(), signer.clone(), market_address),
            builder: ComputeRequestBuilder::new(rpc_provider.clone(), signer.address(), market_address, system_id),
            tracker: ComputeRequestTracker::new(rpc_provider, market_address)
        }
    }

    /// sign the inputted proof request and submit it to the taralli server.
    /// then start tracking the request auction and resolution on-chain.
    pub async fn submit_and_track_request(
        &self,
        request: ComputeRequest<SystemParams>,
        auction_time_length: u64,
    ) -> Result<()> {
        // compute request id
        let request_id = compute_request_id(&request.proof_request, &request.signature);

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

        tracing::info!("tracking started for request ID: {}", request_id);
        tracing::info!("submitting request to server");

        // submit signed request to server
        let response = self
            .base
            .api_client
            .submit_intent(request)
            .await
            .map_err(|e| ClientError::ServerRequestError(e.to_string()))?;

        // track the request
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
        // compute witness
        let witness = compute_request_witness(&request.proof_request);
        // build permit2 digest
        let permit2_digest = compute_request_permit2_digest(&request.proof_request, witness);
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

    pub fn validate_request(&self, _request: &ComputeRequest<SystemParams>) -> Result<()> {
        // validate a request built by the requester client
        //request.validate()?;
        Ok(())
    }
}