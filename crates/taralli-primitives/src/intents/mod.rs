//! This module contains the ComputeIntent Implementations used by the protocol.

use crate::systems::{System, SystemParams};
use crate::validation::Validate;
use alloy::primitives::FixedBytes;
use serde::{Deserialize, Serialize};

pub mod offer;
pub mod request;

/// Trait representing common behavior for compute intents
pub trait ComputeIntent: Validate + Serialize + for<'de> Deserialize<'de> + Send + Sync {
    type System: System;
    type ProofCommitment;

    fn type_string(&self) -> String;
    fn compute_id(&self) -> FixedBytes<32>;
    fn compute_permit2_digest(&self) -> FixedBytes<32>;
    fn system_params(&self) -> Option<&SystemParams> {
        self.system().system_params()
    }
}
