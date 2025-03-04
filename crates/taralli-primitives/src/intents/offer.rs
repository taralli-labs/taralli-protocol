use alloy::{
    dyn_abi::DynSolValue,
    primitives::{keccak256, PrimitiveSignature, B256, U256},
    sol_types::SolValue,
};
use serde::{Deserialize, Serialize};

use crate::{
    abi::universal_porchetta::UniversalPorchetta,
    systems::{System, SystemId},
};

use super::ComputeIntent;

/// Compute offer type generic over all Systems
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(bound = "S: System")]
pub struct ComputeOffer<S: System> {
    pub system_id: SystemId,
    pub system: S,
    pub proof_offer: UniversalPorchetta::ProofOffer,
    pub signature: PrimitiveSignature,
}

impl<S: System> ComputeIntent for ComputeOffer<S> {
    type System = S;
    type ProofCommitment = UniversalPorchetta::ProofOffer;

    fn type_string(&self) -> String {
        "offer".to_string()
    }

    fn compute_id(&self) -> B256 {
        // encode + hash `extraData` and `signature`
        let extra_data_hash = keccak256(self.proof_offer.extraData.abi_encode());
        let signature_hash = keccak256(self.signature.as_bytes().abi_encode());

        // Encode ProofRequest + Signature
        let values = DynSolValue::Tuple(vec![
            DynSolValue::Address(self.proof_offer.market),
            DynSolValue::Uint(self.proof_offer.nonce, 256),
            DynSolValue::Address(self.proof_offer.rewardToken),
            DynSolValue::Uint(self.proof_offer.rewardAmount, 256),
            DynSolValue::Address(self.proof_offer.stakeToken),
            DynSolValue::Uint(self.proof_offer.stakeAmount, 256),
            DynSolValue::Uint(U256::from(self.proof_offer.startAuctionTimestamp), 64),
            DynSolValue::Uint(U256::from(self.proof_offer.endAuctionTimestamp), 64),
            DynSolValue::Uint(U256::from(self.proof_offer.provingTime), 32),
            DynSolValue::FixedBytes(self.proof_offer.inputsCommitment, 32),
            DynSolValue::FixedBytes(extra_data_hash, 32),
            DynSolValue::FixedBytes(signature_hash, 32),
        ]);
        let preimage = values.abi_encode();

        // hash encoded preimage to get request id
        keccak256(&preimage)
    }
}
