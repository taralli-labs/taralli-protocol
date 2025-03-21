use alloy::primitives::PrimitiveSignature;
use serde::{Deserialize, Serialize};

use crate::{
    abi::{
        universal_bombetta::UniversalBombetta::ProofRequest,
        universal_porchetta::UniversalPorchetta::ProofOffer,
    },
    systems::SystemId,
};

/// There's a need for a strip down `ComputeRequest` that doesn't contain the whole `system` data within itself.
/// That so we can more easily send compute request data across the network, given how big `system` can be.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PartialComputeRequest {
    pub system_id: SystemId,
    pub proof_request: ProofRequest,
    pub signature: PrimitiveSignature,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ComputeRequestCompressed {
    pub system_id: SystemId,
    pub system: Vec<u8>,
    pub proof_request: ProofRequest,
    pub signature: PrimitiveSignature,
}

impl From<(PartialComputeRequest, Vec<u8>)> for ComputeRequestCompressed {
    fn from(value: (PartialComputeRequest, Vec<u8>)) -> Self {
        ComputeRequestCompressed {
            system_id: value.0.system_id,
            system: value.1,
            proof_request: value.0.proof_request,
            signature: value.0.signature,
        }
    }
}

/// Same thing for compute offers as above
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PartialComputeOffer {
    pub system_id: SystemId,
    pub proof_offer: ProofOffer,
    pub signature: PrimitiveSignature,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ComputeOfferCompressed {
    pub system_id: SystemId,
    pub system: Vec<u8>,
    pub proof_offer: ProofOffer,
    pub signature: PrimitiveSignature,
}

impl From<(PartialComputeOffer, Vec<u8>)> for ComputeOfferCompressed {
    fn from(value: (PartialComputeOffer, Vec<u8>)) -> Self {
        ComputeOfferCompressed {
            system_id: value.0.system_id,
            system: value.1,
            proof_offer: value.0.proof_offer,
            signature: value.0.signature,
        }
    }
}
