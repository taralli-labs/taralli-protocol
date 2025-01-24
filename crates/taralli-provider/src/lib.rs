pub mod analyzer;
pub mod api;
pub mod bidder;
pub mod builder;
pub mod config;
pub mod error;
pub mod resolver;
pub mod worker;
pub mod workers;

use self::analyzer::RequestAnalyzer;
use self::api::ProviderApi;
use self::bidder::RequestBidder;
use self::config::ProviderConfig;
use self::error::{ProviderError, Result};
use self::resolver::RequestResolver;
use self::worker::{ComputeWorker, WorkResult, WorkerManager};

use builder::ProviderClientBuilder;
use config::{AnalyzerConfig, ApiConfig, BidderConfig, ResolverConfig, ValidationConfig};
use futures_util::StreamExt;
use std::collections::HashMap;
use taralli_primitives::alloy::consensus::BlockHeader;
use taralli_primitives::alloy::eips::Latest;
use taralli_primitives::alloy::primitives::FixedBytes;
use taralli_primitives::alloy::{
    eips::BlockId,
    network::{BlockResponse, BlockTransactionsKind, Network},
    primitives::Address,
    providers::Provider,
    transports::Transport,
};
use taralli_primitives::systems::{ProvingSystemId, ProvingSystemParams};
use taralli_primitives::utils::compute_request_id;
use taralli_primitives::Request;
use url::Url;

pub struct ProviderClient<T, P, N>
where
    T: Transport + Clone,
    P: Provider<T, N> + Clone,
    N: Network + Clone,
{
    config: ProviderConfig<T, P, N>,
    api: ProviderApi,
    analyzer: RequestAnalyzer<T, P, N>,
    bidder: RequestBidder<T, P, N>,
    resolver: RequestResolver<T, P, N>,
    worker_manager: WorkerManager,
}

impl<T, P, N> ProviderClient<T, P, N>
where
    T: Transport + Clone,
    P: Provider<T, N> + Clone,
    N: Network + Clone,
{
    fn _new(
        rpc_provider: P,
        market_address: Address,
        server_url: Url,
        workers: HashMap<ProvingSystemId, Box<dyn ComputeWorker>>,
    ) -> Result<Self> {
        let supported_systems: Vec<_> = workers.keys().cloned().collect();

        // Create base config
        let config = ProviderConfig::new(rpc_provider.clone(), market_address, server_url.clone());

        // Create component configs and instances
        let api = ProviderApi::new(ApiConfig {
            server_url,
            request_timeout: 30,
            max_retries: 3,
        })?;

        let analyzer = RequestAnalyzer::new(
            rpc_provider.clone(),
            AnalyzerConfig {
                market_address,
                supported_proving_systems: supported_systems.clone(),
                validation: ValidationConfig::default(),
            },
        );

        let bidder = RequestBidder::new(
            rpc_provider.clone(),
            BidderConfig {
                market_address,
                min_bid_delay: 0,
                max_bid_attempts: 3,
            },
        );

        let resolver = RequestResolver::new(
            rpc_provider.clone(),
            ResolverConfig {
                market_address,
                confirmation_blocks: 1,
            },
        );

        let worker_manager = WorkerManager::new(workers);

        Ok(Self {
            config,
            api,
            analyzer,
            bidder,
            resolver,
            worker_manager,
        })
    }

    pub fn builder(config: ProviderConfig<T, P, N>) -> ProviderClientBuilder<T, P, N> {
        ProviderClientBuilder::new(config)
    }

    pub async fn run(&self) -> Result<()> {
        let mut stream = self
            .api
            .subscribe_to_markets()
            .await
            .map_err(|e| ProviderError::ServerRequestError(e.to_string()))?;
        tracing::info!("subscribed to markets, waiting for incoming requests");
        while let Some(result) = stream.next().await {
            match result {
                Ok(proof_request) => {
                    let request_id = compute_request_id(
                        &proof_request.onchain_proof_request,
                        proof_request.signature,
                    );
                    tracing::info!(
                        "Incoming proof request - proving system id: {:?}, onchain proof request: {:?}, request ID: {:?}",
                        proof_request.proving_system_id,
                        proof_request.onchain_proof_request,
                        request_id
                    );
                    if let Err(e) = self.process_request(request_id, proof_request).await {
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
        request: Request<ProvingSystemParams>,
    ) -> Result<()> {
        // Fetch latest block timestamp
        // TODO: remove this call from the request processing work flow, instead passing it in as input from another external process
        let current_ts = self
            .config
            .rpc_provider
            .get_block(BlockId::Number(Latest), BlockTransactionsKind::Hashes)
            .await
            .map_err(|e| ProviderError::RpcRequestError(e.to_string()))?
            .ok_or_else(|| ProviderError::RpcRequestError("Block header not found".to_string()))?
            .header()
            .timestamp();

        tracing::info!("latest block timesetamp fetched: {}", current_ts);

        // analyze the validity and profitability of the request
        self.analyzer
            .analyze(&request, current_ts)
            .map_err(|e| ProviderError::RequestAnalysisError(e.to_string()))?;
        tracing::info!("analysis done");

        // Submit a bid for the request
        self.bidder
            .submit_bid(
                request.onchain_proof_request.clone(),
                request.signature,
                request.onchain_proof_request.minRewardAmount,
                current_ts,
            )
            .await
            .map_err(|e| ProviderError::TransactionFailure(format!("bid txs failed: {}", e)))?;

        tracing::info!("bid transaction submitted");

        // Execute worker
        let work_result: WorkResult = self
            .worker_manager
            .execute(&request)
            .await
            .map_err(|e| ProviderError::WorkerExecutionFailed(e.to_string()))?;

        tracing::info!("worker executed");

        // Resolve request
        self.resolver
            .resolve_request(
                request_id,
                work_result.opaque_submission,
                work_result.partial_commitment,
            )
            .await
            .map_err(|e| ProviderError::TransactionFailure(format!("resolve txs failed: {}", e)))?;

        tracing::info!("resolve transaction submitted");

        Ok(())
    }
}
