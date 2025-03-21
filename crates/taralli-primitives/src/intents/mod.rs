//! This module contains the ComputeIntent Implementations used by the protocol.

use crate::systems::{System, SystemId};
use alloy::primitives::{Address, FixedBytes, PrimitiveSignature, U256};
use serde::{Deserialize, Serialize};

pub mod offer;
pub mod request;

// Common trait for shared fields across all intent type's proof commitment structures
pub trait CommonProofCommitment: Serialize + for<'de> Deserialize<'de> + Send + Sync {
    fn market(&self) -> &Address;
    fn nonce(&self) -> &U256;
    fn start_auction_timestamp(&self) -> u64;
    fn end_auction_timestamp(&self) -> u64;
    fn proving_time(&self) -> u32;
    fn inputs_commitment(&self) -> FixedBytes<32>;
}

/// Trait representing common behavior for compute intents
pub trait ComputeIntent: Serialize + for<'de> Deserialize<'de> + Send + Sync {
    type System: System;
    type ProofCommitment: CommonProofCommitment;

    /// Compute Intent data
    fn system_id(&self) -> SystemId;
    fn system(&self) -> &impl System;
    fn proof_commitment(&self) -> &Self::ProofCommitment;
    fn signature(&self) -> &PrimitiveSignature;

    /// utility methods
    // type string associated to this intent type
    fn type_string(&self) -> String;
    // compute intent id
    fn compute_id(&self) -> FixedBytes<32>;
    // compute permit2 digest for intent signing
    fn compute_permit2_digest(&self) -> FixedBytes<32>;
}
