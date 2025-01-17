use std::collections::HashMap;
use taralli_primitives::{
    alloy::{network::Network, providers::Provider, transports::Transport},
    taralli_systems::id::ProvingSystemId,
};

use crate::{
    analyzer::RequestAnalyzer,
    api::ProviderApi,
    bidder::RequestBidder,
    config::{
        AnalyzerConfig, ApiConfig, BidderConfig, ProviderConfig, ResolverConfig, ValidationConfig,
    },
    error::{ProviderError, Result},
    resolver::RequestResolver,
    worker::{ComputeWorker, WorkerManager},
    ProviderClient,
};

pub struct ProviderClientBuilder<T, P, N>
where
    T: Transport + Clone,
    P: Provider<T, N> + Clone,
    N: Network + Clone,
{
    config: ProviderConfig<T, P, N>,
    workers: HashMap<ProvingSystemId, Box<dyn ComputeWorker>>,
    validation_config: ValidationConfig,
    bidder_config: BidderConfig,
    resolver_config: ResolverConfig,
    api_config: ApiConfig,
}

impl<T, P, N> ProviderClientBuilder<T, P, N>
where
    T: Transport + Clone,
    P: Provider<T, N> + Clone,
    N: Network + Clone,
{
    pub fn new(config: ProviderConfig<T, P, N>) -> Self {
        // Initialize with defaults but override shared values
        let bidder_config = BidderConfig {
            market_address: config.market_address,
            ..Default::default()
        };
        let resolver_config = ResolverConfig {
            market_address: config.market_address,
            ..Default::default()
        };
        let api_config = ApiConfig {
            server_url: config.server_url.clone(),
            ..Default::default()
        };
        Self {
            config,
            workers: HashMap::new(),
            validation_config: ValidationConfig::default(),
            bidder_config,
            resolver_config,
            api_config,
        }
    }

    /// Register a worker for a specific proving system
    pub fn with_worker<W: ComputeWorker + 'static>(
        mut self,
        proving_system: impl TryInto<ProvingSystemId, Error = String>,
        worker: W,
    ) -> Result<Self> {
        let id = proving_system
            .try_into()
            .map_err(|e| ProviderError::BuilderError(e.to_string()))?;

        self.workers.insert(id, Box::new(worker));
        Ok(self)
    }

    // Optional configuration methods
    pub fn with_validation_config(
        &mut self,
        minimum_allowed_proving_time: u32,
        maximum_start_delay: u32,
        maximum_allowed_stake: u128,
    ) -> &mut Self {
        self.validation_config = ValidationConfig {
            minimum_allowed_proving_time,
            maximum_start_delay,
            maximum_allowed_stake,
        };
        self
    }

    pub fn with_bidder_config(&mut self, min_bid_delay: u64, max_bid_attempts: u32) -> &mut Self {
        self.bidder_config.min_bid_delay = min_bid_delay;
        self.bidder_config.max_bid_attempts = max_bid_attempts;
        self
    }

    pub fn with_resolver_config(&mut self, confirmation_blocks: u64) -> &mut Self {
        self.resolver_config.confirmation_blocks = confirmation_blocks;
        self
    }

    pub fn with_api_config(&mut self, request_timeout: u64, max_retries: u32) -> &mut Self {
        self.api_config.request_timeout = request_timeout;
        self.api_config.max_retries = max_retries;
        self
    }

    pub fn build(self) -> ProviderClient<T, P, N> {
        if self.workers.is_empty() {
            panic!("No workers registered. Provider must support at least one system.");
        }

        let supported_systems: Vec<_> = self.workers.keys().cloned().collect();

        // Create analyzer config with supported systems from workers
        let analyzer_config = AnalyzerConfig {
            market_address: self.config.market_address,
            supported_proving_systems: supported_systems,
            validation: self.validation_config,
        };

        // Create components
        let api = ProviderApi::new(self.api_config);
        let analyzer = RequestAnalyzer::new(self.config.rpc_provider.clone(), analyzer_config);
        let bidder = RequestBidder::new(self.config.rpc_provider.clone(), self.bidder_config);
        let resolver = RequestResolver::new(self.config.rpc_provider.clone(), self.resolver_config);
        let worker_manager = WorkerManager::new(self.workers);

        ProviderClient {
            config: self.config,
            api,
            analyzer,
            bidder,
            resolver,
            worker_manager,
        }
    }
}
