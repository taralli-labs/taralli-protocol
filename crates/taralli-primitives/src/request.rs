use crate::abi::universal_bombetta::UniversalBombetta;
use crate::systems::{ProvingSystemId, ProvingSystemInformation};
use alloy::primitives::PrimitiveSignature;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

/// Type cast sol! rust representation of UniversalBombetta.sol's ProofRequest type for clarity
pub type OnChainProofRequest = UniversalBombetta::ProofRequest;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Request<I: ProvingSystemInformation> {
    pub proving_system_id: ProvingSystemId,
    pub proving_system_information: I,
    pub onchain_proof_request: OnChainProofRequest,
    pub signature: PrimitiveSignature,
}
