use std::{collections::HashMap, sync::Arc};

use futures_util::StreamExt;
use taralli_primitives::alloy::{
    consensus::BlockHeader,
    eips::{BlockId, BlockNumberOrTag::Latest},
    network::{BlockResponse, BlockTransactionsKind, Network},
    primitives::{Address, FixedBytes},
    providers::Provider,
    signers::Signer,
    transports::Transport,
};
use taralli_primitives::{
    intents::{request::ComputeRequest, ComputeIntent},
    systems::{SystemId, SystemParams},
    validation::{
        registry::ValidatorRegistry,
        request::{ComputeRequestValidator, RequestValidationConfig},
    },
};

use url::Url;

use crate::error::{ClientError, Result};
use crate::{
    analyzer::{request::ComputeRequestAnalyzer, IntentAnalyzer},
    bidder::{request::ComputeRequestBidParams, request::ComputeRequestBidder, IntentBidder},
    resolver::{request::ComputeRequestResolver, IntentResolver},
    worker::{ComputeWorker, WorkResult, WorkerManager},
};
use crate::{api::subscribe::SubscribeApiClient, client::BaseClient};

/// Client that fulfills `ComputeRequests` by subscribing to the protocol server over websocket
/// stream to receive newly submitted `ComputeRequests` at the given system IDs they subscribed to.
/// It then processes the incoming compute requests, bids upon them, compute's the requested compute
/// workload and then resolves the compute request within the market contract.
pub struct ProviderStreamingClient<T, P, N, S>
where
    T: Transport + Clone,
    P: Provider<T, N> + Clone,
    N: Network + Clone,
{
    base: BaseClient<T, P, N, S>,
    api: SubscribeApiClient,
    analyzer: ComputeRequestAnalyzer<T, P, N>,
    bidder: ComputeRequestBidder<T, P, N>,
    worker_manager: WorkerManager<ComputeRequest<SystemParams>>,
    resolver: ComputeRequestResolver<T, P, N>,
}

impl<T, P, N, S> ProviderStreamingClient<T, P, N, S>
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
        validation_config: RequestValidationConfig,
    ) -> Self {
        Self {
            base: BaseClient::new(rpc_provider.clone(), signer.clone(), market_address),
            api: SubscribeApiClient::new(server_url.clone(), 0u8),
            analyzer: ComputeRequestAnalyzer::new(
                rpc_provider.clone(),
                market_address,
                validation_config,
            ),
            bidder: ComputeRequestBidder::new(rpc_provider.clone(), market_address),
            worker_manager: WorkerManager::new(HashMap::new()),
            resolver: ComputeRequestResolver::new(rpc_provider, market_address),
        }
    }

    /// Register a system configuration with the client for a specific system
    /// (systemID -> `ComputeWorker` + Validator)
    pub fn with_system_configuration<
        W: ComputeWorker<ComputeRequest<SystemParams>> + Send + Sync + 'static,
    >(
        mut self,
        system_id: SystemId,
        worker: W,
        validator: ComputeRequestValidator,
    ) -> Result<Self> {
        // update api client's system ID bit mask for subscriptions
        // bitwise OR against the existing bit mask
        let updated_mask = self.api.subscribed_to | system_id.as_bit();
        // set api client's system id mask for subscription of this system
        self.api.set_system_id_mask(updated_mask);

        // set compute worker for the system
        self.worker_manager
            .workers
            .insert(system_id, Arc::new(worker));

        // set analyzer/validator for the system
        self.analyzer
            .validator_registry
            .register(system_id, validator);
        Ok(self)
    }

    pub async fn run(&self) -> Result<()> {
        // subscribe to all markets included within the client's system mask
        let mut stream = self
            .api
            .subscribe_to_markets()
            .await
            .map_err(|e| ClientError::ServerRequestError(e.to_string()))?;
        tracing::info!("subscribed to markets, waiting for incoming requests");

        while let Some(result) = stream.next().await {
            match result {
                Ok(request) => {
                    let request_id = request.compute_id();
                    tracing::info!(
                        "Incoming request - proving system id: {:?}, proof request: {:?}, request ID: {:?}",
                        request.system_id,
                        request.proof_request,
                        request_id
                    );
                    if let Err(e) = self.process_request(request_id, request).await {
                        tracing::error!("Failed to process proof request: {:?}", e);
                    }
                }
                Err(e) => tracing::error!("Error receiving event: {:?}", e),
            }
            tracing::info!("request processed");
        }

        Ok(())
    }

    async fn process_request(
        &self,
        request_id: FixedBytes<32>,
        request: ComputeRequest<SystemParams>,
    ) -> Result<()> {
        // Fetch latest block timestamp
        // TODO: remove this call from the request processing work flow, instead passing it in as input from another external process
        let current_ts = self
            .base
            .rpc_provider
            .get_block(BlockId::Number(Latest), BlockTransactionsKind::Hashes)
            .await
            .map_err(|e| ClientError::RpcRequestError(e.to_string()))?
            .ok_or_else(|| ClientError::RpcRequestError("Block header not found".to_string()))?
            .header()
            .timestamp();

        tracing::info!("latest block timesetamp fetched: {}", current_ts);

        // analyze the validity and profitability of the request
        self.analyzer
            .analyze(current_ts, &request)
            .await
            .map_err(|e| ClientError::IntentAnalysisError(e.to_string()))?;
        tracing::info!("analysis done");

        // for now hard code minimum value since analysis is incomplete
        let bid_params = ComputeRequestBidParams {
            target_amount: request.proof_request.minRewardAmount,
        };

        // Submit a bid for the request
        self.bidder
            .submit_bid(
                current_ts,
                request_id,
                bid_params,
                request.proof_request.clone(),
                request.signature,
            )
            .await
            .map_err(|e| ClientError::TransactionFailure(format!("bid txs failed: {e}")))?;

        tracing::info!("bid transaction submitted successfully");

        // Execute worker
        let work_result: WorkResult = self
            .worker_manager
            .execute(&request)
            .await
            .map_err(|e| ClientError::WorkerError(e.to_string()))?;

        tracing::info!("worker executed");

        // Resolve request
        self.resolver
            .resolve_intent(request_id, work_result.opaque_submission)
            .await
            .map_err(|e| ClientError::TransactionFailure(format!("resolve txs failed: {e}")))?;

        tracing::info!("resolve transaction submitted");

        Ok(())
    }
}
