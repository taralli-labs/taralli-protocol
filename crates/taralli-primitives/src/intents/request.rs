use alloy::{
    dyn_abi::DynSolValue,
    primitives::{keccak256, PrimitiveSignature, B256, U256},
    sol_types::SolValue,
};
use serde::{Deserialize, Serialize};

use crate::{
    abi::universal_bombetta::UniversalBombetta,
    systems::{System, SystemId},
};

use super::ComputeIntent;

/// Compute request type generic over all Systems
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(bound = "S: System")]
pub struct ComputeRequest<S: System> {
    pub system_id: SystemId,
    pub system: S,
    pub proof_request: UniversalBombetta::ProofRequest,
    pub signature: PrimitiveSignature,
}

impl<S: System> ComputeIntent for ComputeRequest<S> {
    type System = S;
    type ProofCommitment = UniversalBombetta::ProofRequest;

    fn compute_id(&self) -> B256 {
        // encode + hash `extraData` and `signature`
        let extra_data_hash = keccak256(self.proof_request.extraData.abi_encode());
        let signature_hash = keccak256(self.signature.as_bytes().abi_encode());

        // Encode OnChainProofRequest + Signature
        let values = DynSolValue::Tuple(vec![
            DynSolValue::Address(self.proof_request.market),
            DynSolValue::Uint(self.proof_request.nonce, 256),
            DynSolValue::Address(self.proof_request.rewardToken),
            DynSolValue::Uint(self.proof_request.maxRewardAmount, 256),
            DynSolValue::Uint(self.proof_request.minRewardAmount, 256),
            DynSolValue::Uint(U256::from(self.proof_request.minimumStake), 128),
            DynSolValue::Uint(U256::from(self.proof_request.startAuctionTimestamp), 64),
            DynSolValue::Uint(U256::from(self.proof_request.endAuctionTimestamp), 64),
            DynSolValue::Uint(U256::from(self.proof_request.provingTime), 32),
            DynSolValue::FixedBytes(self.proof_request.inputsCommitment, 32),
            DynSolValue::FixedBytes(extra_data_hash, 32),
            DynSolValue::FixedBytes(signature_hash, 32),
        ]);
        let preimage = values.abi_encode();

        // hash encoded preimage to get request id
        keccak256(&preimage)
    }
}
