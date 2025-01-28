use crate::abi::universal_bombetta::UniversalBombetta;
use crate::systems::{ProvingSystem, ProvingSystemId};
use alloy::primitives::PrimitiveSignature;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ComputeRequest<P: ProvingSystem> {
    pub proving_system_id: ProvingSystemId,
    pub proving_system: P,
    pub proof_request: UniversalBombetta::ProofRequest,
    pub signature: PrimitiveSignature,
}
