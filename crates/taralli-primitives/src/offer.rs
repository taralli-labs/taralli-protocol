use crate::abi::universal_porchetta::UniversalPorchetta;
use crate::systems::{ProvingSystem, ProvingSystemId};
use alloy::primitives::PrimitiveSignature;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ComputeOffer<P: ProvingSystem> {
    pub proving_system_id: ProvingSystemId,
    pub proving_system: P,
    pub proof_offer: UniversalPorchetta::ProofOffer,
    pub signature: PrimitiveSignature,
}
