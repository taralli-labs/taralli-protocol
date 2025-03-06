use alloy::{
    dyn_abi::DynSolValue,
    primitives::{keccak256, FixedBytes, PrimitiveSignature, B256, U256},
    sol_types::SolValue,
};
use serde::{Deserialize, Serialize};

use crate::{
    abi::{permit2::ISignatureTransfer::TokenPermissions, universal_bombetta::UniversalBombetta},
    systems::{System, SystemId},
    utils::{
        hash_typed_data, PERMIT2_DOMAIN_SEPARATOR, PERMIT_TRANSFER_FROM_WITNESS_TYPEHASH_STUB,
        TOKEN_PERMISSIONS_TYPE_HASH,
    },
};
use lazy_static::lazy_static;

use super::ComputeIntent;

// bombetta
pub const FULL_PROOF_REQUEST_WITNESS_TYPE_STRING_STUB: &str =
    "ProofRequest witness)TokenPermissions(address token,uint256 amount)ProofRequest(address signer,address market,uint256 nonce,address rewardToken,uint256 maxRewardAmount,uint256 minRewardAmount,uint128 minimumStake,uint64 startAuctionTimestamp,uint64 endAuctionTimestamp,uint32 provingTime,bytes32 inputsCommitment,bytes extraData)";
pub const PROOF_REQUEST_WITNESS_TYPE_STRING: &str =
    "ProofRequest(address signer,address market,uint256 nonce,address rewardToken,uint256 maxRewardAmount,uint256 minRewardAmount,uint128 minimumStake,uint64 startAuctionTimestamp,uint64 endAuctionTimestamp,uint32 provingTime,bytes32 inputsCommitment,bytes extraData)";

lazy_static! {
    pub static ref REQUEST_PERMIT_TYPE_HASH: B256 = {
        // craft preimage
        let type_hash_preimage = [
            PERMIT_TRANSFER_FROM_WITNESS_TYPEHASH_STUB.as_bytes(),
            FULL_PROOF_REQUEST_WITNESS_TYPE_STRING_STUB.as_bytes(),
        ]
        .concat();
        // Compute hash
        keccak256(&type_hash_preimage)
    };
    pub static ref PROOF_REQUEST_WITNESS_TYPE_HASH: B256 =
        keccak256(PROOF_REQUEST_WITNESS_TYPE_STRING.as_bytes());
}

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

    fn type_string(&self) -> String {
        "request".to_string()
    }

    fn compute_id(&self) -> FixedBytes<32> {
        // encode + hash `extraData` and `signature`
        let extra_data_hash = keccak256(self.proof_request.extraData.abi_encode());
        let signature_hash = keccak256(self.signature.as_bytes().abi_encode());

        // Encode OnChainProofRequest + Signature
        let values = DynSolValue::Tuple(vec![
            DynSolValue::Address(self.proof_request.signer),
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

    fn compute_permit2_digest(&self) -> FixedBytes<32> {
        // compute witness
        let extra_data_hash = keccak256(&self.proof_request.extraData);

        let request_witness_values = DynSolValue::Tuple(vec![
            DynSolValue::FixedBytes(*PROOF_REQUEST_WITNESS_TYPE_HASH, 32),
            DynSolValue::Address(self.proof_request.signer),
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
        ]);

        // hash encoded witness
        let witness = keccak256(request_witness_values.abi_encode());

        // compute permit2 digest
        // encode token permissions data
        let token_permissions = TokenPermissions {
            token: self.proof_request.rewardToken,
            amount: self.proof_request.maxRewardAmount,
        };
        let token_permissions_bytes = token_permissions.abi_encode();
        //println!("token_permissions_bytes: {:?}", token_permissions_bytes);

        let token_permissions_hash_preimage = [
            TOKEN_PERMISSIONS_TYPE_HASH.abi_encode(),
            token_permissions_bytes,
        ]
        .concat();

        //println!("COMPUTE REQUEST PERMIT2 DIGEST: token permissions pre image: {:?}", token_permissions_hash_preimage);
        //println!("COMPUTE REQUEST PERMIT2 DIGEST: token permissions hash: {:?}", keccak256(&token_permissions_hash_preimage));

        // hash token permissions encoding
        let token_permissions_hash = keccak256(&token_permissions_hash_preimage);
        println!("token_permissions_hash: {}", token_permissions_hash);

        // encode data hash preimage
        let data_hash_preimage = DynSolValue::Tuple(vec![
            DynSolValue::FixedBytes(*REQUEST_PERMIT_TYPE_HASH, 32),
            DynSolValue::FixedBytes(token_permissions_hash, 32),
            DynSolValue::Address(self.proof_request.market),
            DynSolValue::Uint(self.proof_request.nonce, 256),
            DynSolValue::Uint(U256::from(self.proof_request.endAuctionTimestamp), 64),
            DynSolValue::FixedBytes(witness, 32),
        ])
        .abi_encode();

        //println!("COMPUTE REQUEST PERMIT2 DIGEST: data hash pre image: {:?}", data_hash_preimage);
        //println!(
        //    "COMPUTE REQUEST PERMIT2 DIGEST: data hash: {:?}",
        //    keccak256(&data_hash_preimage)
        //);

        // hash data hash encoding
        let data_hash = keccak256(&data_hash_preimage);

        // return the signable eip712 permit2 hash
        hash_typed_data(PERMIT2_DOMAIN_SEPARATOR, data_hash)
    }
}
