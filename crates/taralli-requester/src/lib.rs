pub mod api;
pub mod builder;
pub mod config;
pub mod error;
mod nonce_manager;
pub mod tracker;

use self::api::RequesterApi;
use self::builder::RequestBuilder;
use self::config::RequesterConfig;
use self::error::{RequesterError, Result};
use self::tracker::RequestTracker;
use std::time::Duration;
use taralli_primitives::alloy::primitives::PrimitiveSignature;
use taralli_primitives::alloy::{
    network::Network, providers::Provider, signers::Signer, transports::Transport,
};
use taralli_primitives::systems::ProvingSystemParams;
use taralli_primitives::utils::{
    compute_permit2_digest, compute_request_id, compute_request_witness,
};
use taralli_primitives::validation::validate_request;
use taralli_primitives::Request;

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
            config.proving_system_id,
        );
        let tracker = RequestTracker::new(config.rpc_provider.clone(), config.market_address);

        Self {
            config,
            api,
            builder,
            tracker,
        }
    }

    /// sign the inputted proof request and submit it to the taralli server.
    /// then start tracking the request auction and resolution on-chain.
    pub async fn submit_and_track_request(
        &self,
        request: Request<ProvingSystemParams>,
        auction_time_length: u64,
    ) -> Result<()> {
        // compute request id
        let request_id = compute_request_id(&request.onchain_proof_request, request.signature);

        // compute resolve deadline timestamp
        let resolve_deadline = request.onchain_proof_request.endAuctionTimestamp
            + request.onchain_proof_request.provingTime as u64;

        // setup tracking
        let auction_tracker = self
            .tracker
            .start_auction_tracking(request_id, Duration::from_secs(auction_time_length));
        let resolution_tracker = self
            .tracker
            .start_resolution_tracking(request_id, Duration::from_secs(resolve_deadline));

        tracing::info!("tracking started for request ID: {}", request_id);
        tracing::info!("submitting request to server");

        // submit signed request to server
        let response = self
            .api
            .submit_request(request)
            .await
            .map_err(|e| RequesterError::ServerRequestError(e.to_string()))?;

        // track the request
        if !response.status().is_success() {
            // Parse the error response
            let error_body = response.json::<serde_json::Value>().await.map_err(|e| {
                RequesterError::ServerRequestError(format!("Failed to parse error response: {}", e))
            })?;

            return Err(RequesterError::RequestSubmissionFailed(format!(
                "Server validation failed: {}",
                error_body["error"].as_str().unwrap_or("Unknown error")
            )));
        }

        tracing::info!("Request submitted successfully, waiting for auction result");

        // Wait for auction result
        let _auction_result = auction_tracker
            .await
            .map_err(|e| RequesterError::TrackRequestError(e.to_string()))?
            .ok_or(RequesterError::AuctionTimeoutError())?;

        tracing::info!("Auction completed, waiting for resolution");

        // Wait for resolution
        let _resolution_result = resolution_tracker
            .await
            .map_err(|e| RequesterError::TrackRequestError(e.to_string()))?;

        tracing::info!("Tracking complete");
        Ok(())
    }

    pub async fn sign_request(
        &self,
        mut request: Request<ProvingSystemParams>,
    ) -> Result<Request<ProvingSystemParams>> {
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

    pub fn validate_request(&self, request: &Request<ProvingSystemParams>) -> Result<()> {
        // validate a request built by the requester client
        let dummy_supported_proving_systems = &[request.proving_system_id];
        // NOTE: The latest timestamp check as well as supported proving system checks are both no ops as it is assumed
        //       the requester client is aware of these requirements generally when using the protocol.
        validate_request(
            request,
            request.onchain_proof_request.startAuctionTimestamp - 100,
            &self.config.market_address,
            self.config.validation.minimum_allowed_proving_time,
            self.config.validation.maximum_start_delay,
            self.config.validation.maximum_allowed_stake,
            dummy_supported_proving_systems,
        )?;

        Ok(())
    }
}

/// create dummy ECDSA signature
pub fn create_dummy_signature() -> PrimitiveSignature {
    PrimitiveSignature::try_from(&DUMMY_SIGNATURE_BYTES[..]).unwrap()
}

/// Dummy signature bytes used as placeholder before signing
pub const DUMMY_SIGNATURE_BYTES: [u8; 65] = [
    132, 12, 252, 87, 40, 69, 245, 120, 110, 112, 41, 132, 194, 165, 130, 82, 140, 173, 75, 73,
    178, 161, 11, 157, 177, 190, 127, 202, 144, 5, 133, 101, 37, 231, 16, 156, 235, 152, 22, 141,
    149, 176, 155, 24, 187, 246, 182, 133, 19, 14, 5, 98, 242, 51, 135, 125, 73, 43, 148, 238, 224,
    197, 182, 209, 0, // v value (false/0)
];
