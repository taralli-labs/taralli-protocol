use crate::abi::universal_bombetta::UniversalBombetta;
use crate::systems::{ProvingSystemId, ProvingSystemInformation};
use alloy::primitives::PrimitiveSignature;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ComputeRequest<I: ProvingSystemInformation> {
    pub proving_system_id: ProvingSystemId,
    pub proving_system_information: I,
    pub onchain_proof_request: UniversalBombetta::ProofRequest,
    pub signature: PrimitiveSignature,
}
