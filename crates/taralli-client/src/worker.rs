use crate::error::{ClientError, Result};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use taralli_primitives::alloy::primitives::{Bytes, FixedBytes};
use taralli_primitives::intents::ComputeIntent;
use taralli_primitives::systems::SystemId;

/// Output type of a compute worker that can be used by an intent
/// resolver to resolve a compute intent.
#[derive(Debug)]
pub struct WorkResult {
    pub opaque_submission: Bytes,
    pub partial_commitment: FixedBytes<32>,
}

/// core compute worker trait used by provider clients to
/// run the computation needed to fulfill a compute intent's
/// computational task
#[async_trait]
pub trait ComputeWorker<I: ComputeIntent>: Send + Sync {
    async fn execute(&self, intent: &I) -> Result<WorkResult>;
}

/// manager type allowing clients to handle multiple compute workers organized
/// by system ID to provide compute for many systems simultaneously
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
