use std::collections::HashMap;

use alloy::{consensus::BlockHeader, eips::{BlockId, BlockNumberOrTag::Latest}, network::{BlockResponse, Network}, primitives::{Address, FixedBytes}, providers::Provider, rpc::types::BlockTransactionsKind, signers::Signer, transports::Transport};
use futures_util::StreamExt;
use taralli_primitives::{intents::{ComputeIntent, ComputeRequest}, systems::{SystemId, SystemParams}, utils::compute_request_id, validation::ValidationConfig};
//use taralli_primitives::validation::Validate;

use url::Url;

use crate::{analyzer::{GenericAnalyzer, IntentAnalyzer}, bidder::{ComputeRequestBidParams, ComputeRequestBidder, IntentBidder}, resolver::{ComputeRequestResolver, IntentResolver}, worker::{ComputeWorker, WorkResult, WorkerManager}};
use crate::error::{Result, ClientError};
use super::BaseClient;

pub struct ProviderStreamingClient<T, P, N, S, I, C>
where
    T: Transport + Clone,
    P: Provider<T, N> + Clone,
    N: Network + Clone,
    I: ComputeIntent
{
    base: BaseClient<T, P, N, S>,
    analyzer: GenericAnalyzer<T, P, N, I, C>,
    bidder: ComputeRequestBidder<T, P, N>,
    worker_manager: WorkerManager<I>,
    resolver: ComputeRequestResolver<T, P, N>,
}

impl<T, P, N, S, I, C> ProviderStreamingClient<T, P, N, S, I, C>
where
    T: Transport + Clone,
    P: Provider<T, N> + Clone,
    N: Network + Clone,
    S: Signer + Clone,
    I: ComputeIntent,
    C: ValidationConfig + Send + Sync + Clone,
    I::Config: From<C>,
{
    pub fn new(
        server_url: Url,
        rpc_provider: P,
        signer: S,
        market_address: Address,
        validation_config: C,
        min_bid_delay: u64, 
        max_bid_atempts: u32,
        workers: HashMap<SystemId, Box<dyn ComputeWorker<I>>>,
    ) -> Self {
        Self {
            base: BaseClient::new(server_url, rpc_provider.clone(), signer.clone(), market_address),
            analyzer: GenericAnalyzer::new(rpc_provider.clone(), market_address, validation_config),
            bidder: ComputeRequestBidder::new(rpc_provider.clone(), market_address, min_bid_delay, max_bid_atempts),
            worker_manager: WorkerManager::new(workers),
            resolver: ComputeRequestResolver::new(rpc_provider, market_address),
        }
    }

    pub async fn run(&self) -> Result<()> {
        let mut stream = self
            .base
            .api_client
            .subscribe_to_markets(system_ids)
            .await
            .map_err(|e| ClientError::ServerRequestError(e.to_string()))?;
        tracing::info!("subscribed to markets, waiting for incoming requests");
        while let Some(result) = stream.next().await {
            match result {
                Ok(request) => {
                    let request_id = compute_request_id(&request.proof_request, &request.signature);
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
            target_amount: request.proof_request.minRewardAmount
        };

        // Submit a bid for the request
        self.bidder
            .submit_bid(
                current_ts,
                bid_params,
                request.proof_request.clone(),
                request.signature,
            )
            .await
            .map_err(|e| ClientError::TransactionFailure(format!("bid txs failed: {}", e)))?;

        tracing::info!("bid transaction submitted");

        // Execute worker
        let work_result: WorkResult = self
            .worker_manager
            .execute(&request)
            .await
            .map_err(|e| ClientError::WorkerExecutionFailed(e.to_string()))?;

        tracing::info!("worker executed");

        // Resolve request
        self.resolver
            .resolve_intent(
                request_id,
                work_result.opaque_submission,
            )
            .await
            .map_err(|e| ClientError::TransactionFailure(format!("resolve txs failed: {}", e)))?;

        tracing::info!("resolve transaction submitted");

        Ok(())
    }
}