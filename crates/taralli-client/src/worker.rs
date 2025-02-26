use crate::error::{ClientError, Result};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use taralli_primitives::alloy::primitives::{Bytes, FixedBytes};
use taralli_primitives::intents::ComputeIntent;
use taralli_primitives::systems::SystemId;

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

#[derive(Clone)]
pub struct WorkerManager<I: ComputeIntent> {
    pub workers: HashMap<SystemId, Arc<dyn ComputeWorker<I> + Send + Sync>>,
}

impl<I: ComputeIntent> WorkerManager<I> {
    pub fn new(
        workers: HashMap<SystemId, Arc<dyn ComputeWorker<I> + Send + Sync + 'static>>,
    ) -> Self {
        Self { workers }
    }

    pub async fn execute(&self, intent: &I) -> Result<WorkResult> {
        let worker = self.workers.get(&I::system_id(intent)).ok_or_else(|| {
            ClientError::WorkerError(format!(
                "worker not set for proving system id: {:?}",
                I::system_id(intent)
            ))
        })?;

        worker.execute(intent).await
    }
}
