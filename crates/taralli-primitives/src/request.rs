use crate::abi::universal_bombetta::UniversalBombetta;
use alloy::primitives::PrimitiveSignature;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use taralli_systems::{id::ProvingSystemId, ProvingSystemInformation};

/// Type cast sol! rust representation of UniversalBombetta.sol's ProofRequest type for clarity
pub type OnChainProofRequest = UniversalBombetta::ProofRequest;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProofRequest<I: ProvingSystemInformation> {
    pub proving_system_id: ProvingSystemId,
    pub proving_system_information: I,
    pub onchain_proof_request: OnChainProofRequest,
    pub signature: PrimitiveSignature,
}
