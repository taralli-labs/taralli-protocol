use crate::systems::{System, SystemParams};
use crate::validation::Validate;
use alloy::primitives::B256;
use serde::{Deserialize, Serialize};

pub mod offer;
pub mod request;

/// Compute Intent types
/// Trait representing common behavior for compute intents
pub trait ComputeIntent: Validate + Serialize + for<'de> Deserialize<'de> + Send + Sync {
    type System: System;
    type ProofCommitment;

    fn compute_id(&self) -> B256;
    fn system_params(&self) -> Option<&SystemParams> {
        self.system().system_params()
    }
}
