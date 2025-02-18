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

/// There's a need for a strip down `Request` that doesn't contain the whole `proving_system_information` within itself.
/// That so we can more easily send request data across the network, given how big `proving_system_information` can be.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PartialRequest {
    pub proving_system_id: ProvingSystemId,
    pub onchain_proof_request: OnChainProofRequest,
    pub signature: PrimitiveSignature,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RequestCompressed {
    pub proving_system_id: ProvingSystemId,
    pub proving_system_information: Vec<u8>,
    pub onchain_proof_request: OnChainProofRequest,
    pub signature: PrimitiveSignature,
}

impl From<(PartialRequest, Vec<u8>)> for RequestCompressed {
    fn from(value: (PartialRequest, Vec<u8>)) -> Self {
        RequestCompressed {
            proving_system_id: value.0.proving_system_id,
            proving_system_information: value.1,
            onchain_proof_request: value.0.onchain_proof_request,
            signature: value.0.signature,
        }
    }
}
