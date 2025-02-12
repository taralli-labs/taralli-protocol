use crate::abi::universal_bombetta::UniversalBombetta;
use crate::abi::universal_porchetta::UniversalPorchetta;
use crate::systems::{ProvingSystem, ProvingSystemId};
use alloy::primitives::PrimitiveSignature;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

/// Compute Intent types

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ComputeRequest<P: ProvingSystem> {
    pub proving_system_id: ProvingSystemId,
    pub proving_system: P,
    pub proof_request: UniversalBombetta::ProofRequest,
    pub signature: PrimitiveSignature,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ComputeOffer<P: ProvingSystem> {
    pub proving_system_id: ProvingSystemId,
    pub proving_system: P,
    pub proof_offer: UniversalPorchetta::ProofOffer,
    pub signature: PrimitiveSignature,
}
