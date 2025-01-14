pub mod api;
pub mod builder;
pub mod config;
pub mod error;
mod nonce_manager;
pub mod tracker;

use self::api::RequesterApi;
use self::builder::RequestBuilder;
use self::config::RequesterConfig;
use self::error::{RequesterError, RequesterResult};
use self::tracker::RequestTracker;
use std::time::Duration;
use taralli_primitives::alloy::{
    network::Network, providers::Provider, signers::Signer, transports::Transport,
};
use taralli_primitives::taralli_systems::id::ProvingSystemParams;
use taralli_primitives::utils::{
    compute_permit2_digest, compute_request_id, compute_request_witness,
};
use taralli_primitives::validation::{
    validate_amount_constraints, validate_market_address, validate_signature,
};
use taralli_primitives::ProofRequest;

pub struct RequesterClient<T, P, N, S>
where
    T: Transport + Clone,
    P: Provider<T, N> + Clone,
    N: Network + Clone,
    S: Signer + Clone,
{
    pub config: RequesterConfig<T, P, N, S>,
    pub api: RequesterApi,
    pub builder: RequestBuilder<T, P, N>,
    tracker: RequestTracker<T, P, N>,
}

impl<T, P, N, S> RequesterClient<T, P, N, S>
where
    T: Transport + Clone,
    P: Provider<T, N> + Clone,
    N: Network + Clone,
    S: Signer + Clone,
{
    pub fn new(config: RequesterConfig<T, P, N, S>) -> Self {
        let api = RequesterApi::new(config.taralli_server_url.clone());
        let builder = RequestBuilder::new(
            config.rpc_provider.clone(),
            config.signer.address(),
            config.market_address,
            config.proving_system_id.clone(),
        );
        let tracker = RequestTracker::new(config.rpc_provider.clone(), config.market_address);

        Self {
            config,
            api,
            builder,
            tracker,
        }
    }

    pub fn validate_request(
        &self,
        request: &ProofRequest<ProvingSystemParams>,
        _latest_timestamp: u64,
    ) -> RequesterResult<()> {
        validate_market_address(request, self.config.market_address)?;
        // TODO better design for time constraint checking
        //validate_time_constraints(
        //    latest_timestamp,
        //    self.config.validation.minimum_allowed_proving_time,
        //    self.config.validation.maximum_start_delay,
        //    request,
        //)?;
        validate_amount_constraints(self.config.validation.maximum_allowed_stake, request)?;
        validate_signature(request)?;
        Ok(())
    }

    /// sign the inputted proof request and submit it to the taralli server.
    /// then start tracking the request auction and resolution on-chain.
    pub async fn submit_and_track_request(
        &self,
        proof_request: ProofRequest<ProvingSystemParams>,
        auction_time_length: u64,
    ) -> RequesterResult<()> {
        // compute request id
        let request_id = compute_request_id(
            &proof_request.onchain_proof_request,
            proof_request.signature,
        );

        // compute resolve deadline timestamp
        let resolve_deadline = proof_request.onchain_proof_request.endAuctionTimestamp
            + proof_request.onchain_proof_request.provingTime as u64;

        log::info!("submitting request to server");
        log::info!("request ID: {}", request_id);

        // submit signed request to server
        let response = self
            .api
            .submit_request(proof_request.clone())
            .await
            .map_err(|e| RequesterError::ServerRequestError(e.to_string()))?;

        // track the request
        if response.status().is_success() {
            log::info!("submission success, tracking started");
            self.tracker
                .track_request(
                    request_id,
                    Duration::from_secs(auction_time_length),
                    Duration::from_secs(resolve_deadline),
                )
                .await
                .map_err(|e| RequesterError::TrackRequestError(e.to_string()))?;
            log::info!("tracking complete");
            Ok(())
        } else {
            // Parse the error response
            let error_body = response.json::<serde_json::Value>().await.map_err(|e| {
                RequesterError::ServerRequestError(format!("Failed to parse error response: {}", e))
            })?;

            Err(RequesterError::RequestSubmissionFailed(format!(
                "Server validation failed: {}",
                error_body["error"].as_str().unwrap_or("Unknown error")
            )))
        }
    }

    pub async fn sign_request(
        &self,
        mut request: ProofRequest<ProvingSystemParams>,
    ) -> RequesterResult<ProofRequest<ProvingSystemParams>> {
        // compute witness
        let witness = compute_request_witness(&request.onchain_proof_request);
        // build permit2 digest
        let permit2_digest = compute_permit2_digest(&request.onchain_proof_request, witness);
        // sign permit2 digest
        let signature = self
            .config
            .signer
            .sign_hash(&permit2_digest)
            .await
            .map_err(|e| RequesterError::RequestSigningError(e.to_string()))?;
        // load signature into proof request
        request.signature = signature;
        Ok(request)
    }
}
