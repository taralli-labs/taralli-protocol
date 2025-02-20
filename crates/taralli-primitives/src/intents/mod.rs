use crate::abi::universal_bombetta::UniversalBombetta;
use crate::abi::universal_porchetta::UniversalPorchetta;
use crate::systems::{System, SystemId};
use crate::validation::Validate;
use alloy::primitives::PrimitiveSignature;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

/// Compute Intent types

/// Trait representing common behavior for compute intents
pub trait ComputeIntent: Send + Sync + Serialize + Validate {
    type System: System;
    type ProofCommitment;
}

/// Compute request type generic over all Systems
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ComputeRequest<S: System> {
    pub system_id: SystemId,
    pub system: S,
    pub proof_request: UniversalBombetta::ProofRequest,
    pub signature: PrimitiveSignature,
}

impl<S: System> ComputeIntent for ComputeRequest<S> {
    type System = S;
    type ProofCommitment = UniversalBombetta::ProofRequest;
}

/// Compute offer type generic over all Systems
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ComputeOffer<S: System> {
    pub system_id: SystemId,
    pub system: S,
    pub proof_offer: UniversalPorchetta::ProofOffer,
    pub signature: PrimitiveSignature,
}

impl<S: System> ComputeIntent for ComputeOffer<S> {
    type System = S;
    type ProofCommitment = UniversalPorchetta::ProofOffer;
}
