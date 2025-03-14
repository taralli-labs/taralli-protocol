use std::{collections::HashMap, sync::Arc};

use alloy::{
    consensus::BlockHeader,
    eips::{BlockId, BlockNumberOrTag::Latest},
    network::{BlockResponse, Network},
    primitives::{Address, FixedBytes},
    providers::Provider,
    rpc::types::BlockTransactionsKind,
    signers::Signer,
    transports::Transport,
};
use futures_util::StreamExt;
use taralli_primitives::{
    intents::{request::ComputeRequest, ComputeIntent},
    systems::{SystemId, SystemParams},
    validation::request::{RequestValidationConfig, RequestVerifierConstraints},
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

/// Client that fulfills ComputeRequests by subscribing to the protocol server over websocket
/// stream to receive newly submitted ComputeRequests at the given system IDs they subscribed to.
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
    analyzer: ComputeRequestAnalyzer<T, P, N, ComputeRequest<SystemParams>>,
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
        verifier_constraints: Option<HashMap<SystemId, RequestVerifierConstraints>>,
    ) -> Self {
        Self {
            base: BaseClient::new(rpc_provider.clone(), signer.clone(), market_address),
            api: SubscribeApiClient::new(server_url.clone()),
            analyzer: ComputeRequestAnalyzer::new(
                rpc_provider.clone(),
                market_address,
                validation_config,
                verifier_constraints,
            ),
            bidder: ComputeRequestBidder::new(rpc_provider.clone(), market_address),
            worker_manager: WorkerManager::new(HashMap::new()),
            resolver: ComputeRequestResolver::new(rpc_provider, market_address),
        }
    }

    /// Register a worker for a specific proving system
    pub fn with_worker<W: ComputeWorker<ComputeRequest<SystemParams>> + Send + Sync + 'static>(
        mut self,
        system_id: SystemId,
        worker: W,
    ) -> Result<Self> {
        self.worker_manager
            .workers
            .insert(system_id, Arc::new(worker));
        Ok(self)
    }

    pub async fn run(&self) -> Result<()> {
        // collect system IDs the client has compute worker support for
        let system_ids: Vec<SystemId> = self.worker_manager.workers.keys().cloned().collect();

        // subscribe to all markets the client
        let mut stream = self
            .api
            .subscribe_to_markets(&system_ids)
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
            .map_err(|e| ClientError::TransactionFailure(format!("bid txs failed: {}", e)))?;

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
            .map_err(|e| ClientError::TransactionFailure(format!("resolve txs failed: {}", e)))?;

        tracing::info!("resolve transaction submitted");

        Ok(())
    }
}
