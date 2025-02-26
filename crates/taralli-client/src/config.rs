use std::{collections::HashMap, fmt, marker::PhantomData, sync::Arc};

use alloy::{
    network::Network, primitives::Address, providers::Provider, signers::Signer,
    transports::Transport,
};
use serde::{Deserialize, Serialize};
use taralli_primitives::{
    intents::ComputeIntent,
    systems::SystemId,
    validation::{offer::OfferValidationConfig, request::RequestValidationConfig},
};
use url::Url;

use crate::worker::{ComputeWorker, WorkerManager};

#[derive(Clone)]
pub struct ClientValidationConfigs {
    pub request: RequestValidationConfig,
    pub offer: OfferValidationConfig,
}

// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct AnalyzerConfig {
//     pub market_address: Address,
//     pub validation_config: RequestValidationConfig,
// }

// impl Default for AnalyzerConfig {
//     fn default() -> Self {
//         Self {
//             market_address: Address::ZERO, // Will be set from config
//             validation_config: RequestValidationConfig::default(),
//         }
//     }
// }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BidderConfig {
    pub min_bid_delay: u64,
    pub max_bid_attempts: u32,
}

// impl Default for BidderConfig {
//     fn default() -> Self {
//         Self {
//             market_address: Address::ZERO, // Will be set from config
//             min_bid_delay: 0,
//             max_bid_attempts: 3,
//         }
//     }
// }

// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct ResolverConfig {
//     pub market_address: Address,
//     pub confirmation_blocks: u64,
// }

// impl Default for ResolverConfig {
//     fn default() -> Self {
//         Self {
//             market_address: Address::ZERO, // Will be set from config
//             confirmation_blocks: 1,
//         }
//     }
// }

// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct ApiConfig {
//     pub server_url: Url,
//     pub request_timeout: u64,
//     pub max_retries: u32,
// }

// impl Default for ApiConfig {
//     fn default() -> Self {
//         Self {
//             server_url: Url::parse("http://localhost:8080").unwrap(),
//             request_timeout: 30,
//             max_retries: 3,
//         }
//     }
// }

// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct WorkerConfig {
//     pub supported_proving_systems: Vec<SystemId>,
//     pub max_concurrent_jobs: u32,
// }

// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct RewardTokenConfig {
//     pub address: Address,
//     pub decimal: u8,
// }

// Serializable configs (for loading from files)
#[derive(Clone, Debug, Deserialize)]
pub struct ProviderOfferingConfigFile {
    pub system_id: SystemId,
    pub validation_config: OfferValidationConfig,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ProviderStreamingConfigFile {
    pub supported_systems: Vec<SystemId>,
    pub validation_config: RequestValidationConfig,
}

// Runtime configs (with workers)
#[derive(Clone)]
pub struct ProviderOfferingConfig<I: ComputeIntent> {
    pub system_id: SystemId,
    pub worker: Arc<dyn ComputeWorker<I> + Send + Sync>,
    pub validation_config: OfferValidationConfig,
}

#[derive(Clone)]
pub struct ProviderStreamingConfig<I: ComputeIntent> {
    pub worker_manager: Arc<WorkerManager<I>>,
    pub validation_config: RequestValidationConfig,
}

// Debug implementations
impl<I: ComputeIntent> fmt::Debug for ProviderOfferingConfig<I> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProviderOfferingConfig")
            .field("system_id", &self.system_id)
            .field("validation_config", &self.validation_config)
            .field("worker", &"<ComputeWorker>")
            .finish()
    }
}

impl<I: ComputeIntent> fmt::Debug for ProviderStreamingConfig<I> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProviderStreamingConfig")
            .field("validation_config", &self.validation_config)
            .field("worker_manager", &"<WorkerManager>")
            .finish()
    }
}

// Conversion functions
impl ProviderOfferingConfigFile {
    pub fn to_runtime_config<I: ComputeIntent>(
        &self,
        worker: Arc<dyn ComputeWorker<I> + Send + Sync>,
    ) -> ProviderOfferingConfig<I> {
        ProviderOfferingConfig {
            system_id: self.system_id,
            worker,
            validation_config: self.validation_config.clone(),
        }
    }
}

impl ProviderStreamingConfigFile {
    pub fn to_runtime_config<I: ComputeIntent>(
        &self,
        worker_factory: impl Fn(SystemId) -> Arc<dyn ComputeWorker<I> + Send + Sync>,
    ) -> ProviderStreamingConfig<I> {
        let mut workers = HashMap::new();
        for system_id in &self.supported_systems {
            workers.insert(*system_id, worker_factory(*system_id));
        }

        let worker_manager = Arc::new(WorkerManager::new(workers));

        ProviderStreamingConfig {
            worker_manager,
            validation_config: self.validation_config.clone(),
        }
    }
}

/// requester client configs
// requester searching
#[derive(Clone, Debug, Deserialize)]
pub struct RequesterSearcherConfig {
    pub system_id: SystemId,
    pub validation_config: OfferValidationConfig,
}
#[derive(Clone, Debug, Deserialize)]
pub struct OfferAcceptanceConfig {}
// requester requesting
#[derive(Clone, Debug, Deserialize)]
pub struct RequesterRequestingConfig {
    pub system_id: SystemId,
    pub validation_config: RequestValidationConfig,
}

/// provider client configs
// #[derive(Clone, Debug, Deserialize)]
// pub struct ProviderOfferingConfig<I: ComputeIntent> {
//     pub system_id: SystemId,
//     pub worker: Box<dyn ComputeWorker<ComputeOffer<SystemParams>>>,
//     pub validation_config: OfferValidationConfig
// }

// #[derive(Clone, Debug, Deserialize)]
// pub struct ProviderStreamingConfig<I: ComputeIntent> {
//     pub system_id: SystemId,
//     pub workers: WorkerManager<I>,
//     pub validation_config: OfferValidationConfig
// }

#[derive(Clone, Debug, Deserialize)]
pub struct WorkerConfig {
    pub supported_proving_systems: Vec<SystemId>,
    pub max_concurrent_jobs: u32,
}

// Client modes
#[derive(Clone, Debug, Deserialize)]
pub enum ClientMode {
    Requester(RequesterMode),
    Provider(ProviderMode),
}

// Requester-specific modes
#[derive(Clone, Debug, Deserialize)]
pub enum RequesterMode {
    // Search for existing compute offers
    Searching {
        requester_searcher_config: RequesterSearcherConfig,
    },
    // Create compute requests
    Requesting {
        requester_requesting_config: RequesterRequestingConfig,
    },
}

// Provider-specific modes
#[derive(Clone, Debug, Deserialize)]
pub enum ProviderMode {
    Streaming {
        provider_streaming_config_file: ProviderStreamingConfigFile,
    },
    // Create compute offers
    Offering {
        provider_offering_config_file: ProviderOfferingConfigFile,
    },
}

#[derive(Debug, Clone, Deserialize)]
pub struct ClientConfig<T, P, N, S>
where
    T: Transport + Clone,
    P: Provider<T, N> + Clone,
    N: Network + Clone,
    S: Signer + Clone,
{
    pub rpc_provider: P,
    pub signer: S,
    pub server_url: Url,
    pub market_address: Address,
    pub mode: ClientMode,
    phantom_data: PhantomData<(T, N)>,
}

impl<T, P, N, S> ClientConfig<T, P, N, S>
where
    T: Transport + Clone,
    P: Provider<T, N> + Clone,
    N: Network + Clone,
    S: Signer + Clone,
{
    pub fn new(
        rpc_provider: P,
        signer: S,
        server_url: Url,
        market_address: Address,
        mode: ClientMode,
    ) -> Self {
        Self {
            rpc_provider,
            signer,
            server_url,
            market_address,
            mode,
            phantom_data: PhantomData,
        }
    }
}
