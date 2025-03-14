//! Client Configurations

use std::{collections::HashMap, fmt, sync::Arc};

use serde::{Deserialize, Serialize};
use taralli_primitives::{
    intents::ComputeIntent,
    systems::SystemId,
    validation::{offer::OfferValidationConfig, request::RequestValidationConfig},
};

use crate::worker::{ComputeWorker, WorkerManager};

#[derive(Clone)]
pub struct ClientValidationConfigs {
    pub request: RequestValidationConfig,
    pub offer: OfferValidationConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BidderConfig {
    pub min_bid_delay: u64,
    pub max_bid_attempts: u32,
}

/// Serializable provider configs (for loading from files)
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

/// Runtime provider client configs (with workers)
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

/// provider config Debug impls
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

// Conversion functions for provider configs
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
#[derive(Clone, Debug, Deserialize)]
pub struct RequesterSearcherConfig {
    pub system_id: SystemId,
    pub validation_config: OfferValidationConfig,
}

#[derive(Clone, Debug, Deserialize)]
pub struct RequesterRequestingConfig {
    pub system_id: SystemId,
    pub validation_config: RequestValidationConfig,
}

#[derive(Clone, Debug, Deserialize)]
pub struct WorkerConfig {
    pub supported_proving_systems: Vec<SystemId>,
    pub max_concurrent_jobs: u32,
}
