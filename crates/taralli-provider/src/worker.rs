use crate::error::{ProviderError, Result};
use async_trait::async_trait;
use std::collections::HashMap;
use taralli_primitives::alloy::primitives::{Bytes, FixedBytes};
use taralli_primitives::taralli_systems::id::{ProvingSystemId, ProvingSystemParams};
use taralli_primitives::Request;

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
pub trait ComputeWorker: Send + Sync {
    async fn execute(&self, request: &Request<ProvingSystemParams>) -> Result<WorkResult>;
}

pub(crate) struct WorkerManager {
    workers: HashMap<ProvingSystemId, Box<dyn ComputeWorker>>,
}

impl WorkerManager {
    pub(crate) fn new(workers: HashMap<ProvingSystemId, Box<dyn ComputeWorker>>) -> Self {
        Self { workers }
    }

    pub(crate) async fn execute(
        &self,
        request: &Request<ProvingSystemParams>,
    ) -> Result<WorkResult> {
        let worker = self
            .workers
            .get(&request.proving_system_id)
            .ok_or_else(|| {
                ProviderError::WorkerExecutionFailed(format!(
                    "worker not set for proving system id: {:?}",
                    request.proving_system_id
                ))
            })?;

        worker.execute(request).await
    }
}
