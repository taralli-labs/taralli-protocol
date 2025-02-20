use crate::error::{ClientError, Result};
use async_trait::async_trait;
use std::collections::HashMap;
use taralli_primitives::alloy::primitives::{Bytes, FixedBytes};
use taralli_primitives::intents::{ComputeIntent, ComputeRequest};
use taralli_primitives::systems::{SystemId, SystemParams};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResourceRequirements {
    pub min_memory_mb: u64,
    pub min_cpu_cores: u32,
    pub estimated_runtime_seconds: u64,
    pub gpu_required: bool,
}

#[derive(Debug)]
pub struct WorkResult {
    pub opaque_submission: Bytes,
    pub partial_commitment: FixedBytes<32>,
}

#[async_trait]
pub trait ComputeWorker<I: ComputeIntent>: Send + Sync {
    async fn execute(&self, intent: &I) -> Result<WorkResult>;
}

pub(crate) struct WorkerManager<I: ComputeIntent> {
    pub workers: HashMap<SystemId, Box<dyn ComputeWorker<I>>>,
}

impl<I: ComputeIntent> WorkerManager<I> {
    pub(crate) fn new(workers: HashMap<SystemId, Box<dyn ComputeWorker<I>>>) -> Self {
        Self { workers }
    }

    pub(crate) async fn execute(
        &self,
        intent: &I,
    ) -> Result<WorkResult> {
        let worker = self.workers.get(&<dyn ComputeIntent::system_id(intent)>).ok_or_else(|| {
            ClientError::WorkerExecutionFailed(format!(
                "worker not set for proving system id: {:?}",
                ComputeIntent::system_id(intent)
            ))
        })?;

        worker.execute(intent).await
    }
}
