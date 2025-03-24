use alloy::{
    dyn_abi::DynSolValue,
    primitives::{keccak256, Address, FixedBytes, PrimitiveSignature, B256, U256},
    sol_types::SolValue,
};
use serde::{Deserialize, Serialize};

use crate::{
    abi::{
        permit2::ISignatureTransfer::TokenPermissions,
        universal_bombetta::UniversalBombetta::ProofRequest,
    },
    systems::{System, SystemId},
    utils::{
        hash_typed_data, PERMIT2_DOMAIN_SEPARATOR, PERMIT_TRANSFER_FROM_WITNESS_TYPEHASH_STUB,
        TOKEN_PERMISSIONS_TYPE_HASH,
    },
};
use lazy_static::lazy_static;

use super::{CommonProofCommitment, ComputeIntent};

/// bombetta signature constants
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
    pub proof_request: ProofRequest,
    pub signature: PrimitiveSignature,
}

impl CommonProofCommitment for ProofRequest {
    fn market(&self) -> &Address {
        &self.market
    }

    fn nonce(&self) -> &U256 {
        &self.nonce
    }

    fn start_auction_timestamp(&self) -> u64 {
        self.startAuctionTimestamp
    }

    fn end_auction_timestamp(&self) -> u64 {
        self.endAuctionTimestamp
    }

    fn proving_time(&self) -> u32 {
        self.provingTime
    }

    fn inputs_commitment(&self) -> FixedBytes<32> {
        self.inputsCommitment
    }
}

/// generic compute request implementation
impl<S: System> ComputeIntent for ComputeRequest<S> {
    type System = S;
    type ProofCommitment = ProofRequest;

    fn type_string(&self) -> String {
        "request".to_string()
    }

    fn system_id(&self) -> SystemId {
        self.system_id
    }

    fn system(&self) -> &impl System {
        &self.system
    }

    fn proof_commitment(&self) -> &Self::ProofCommitment {
        &self.proof_request
    }

    fn signature(&self) -> &PrimitiveSignature {
        &self.signature
    }

    fn compute_id(&self) -> FixedBytes<32> {
        compute_request_id(&self.proof_request, &self.signature)
    }

    fn compute_permit2_digest(&self) -> FixedBytes<32> {
        compute_request_permit2_digest(&self.proof_request)
    }
}

pub fn compute_request_id(
    proof_commitment: &ProofRequest,
    signature: &PrimitiveSignature,
) -> FixedBytes<32> {
    // encode + hash `extraData` and `signature`
    let extra_data_hash = keccak256(proof_commitment.extraData.abi_encode());
    let signature_hash = keccak256(signature.as_bytes().abi_encode());

    // Encode ProofRequest + Signature
    let values = DynSolValue::Tuple(vec![
        DynSolValue::Address(proof_commitment.signer),
        DynSolValue::Address(proof_commitment.market),
        DynSolValue::Uint(proof_commitment.nonce, 256),
        DynSolValue::Address(proof_commitment.rewardToken),
        DynSolValue::Uint(proof_commitment.maxRewardAmount, 256),
        DynSolValue::Uint(proof_commitment.minRewardAmount, 256),
        DynSolValue::Uint(U256::from(proof_commitment.minimumStake), 128),
        DynSolValue::Uint(U256::from(proof_commitment.startAuctionTimestamp), 64),
        DynSolValue::Uint(U256::from(proof_commitment.endAuctionTimestamp), 64),
        DynSolValue::Uint(U256::from(proof_commitment.provingTime), 32),
        DynSolValue::FixedBytes(proof_commitment.inputsCommitment, 32),
        DynSolValue::FixedBytes(extra_data_hash, 32),
        DynSolValue::FixedBytes(signature_hash, 32),
    ]);
    let preimage = values.abi_encode();

    // hash encoded preimage to get intent id
    keccak256(&preimage)
}

pub fn compute_request_permit2_digest(proof_commitment: &ProofRequest) -> FixedBytes<32> {
    // compute witness
    let extra_data_hash = keccak256(proof_commitment.extraData.clone());

    let request_witness_values = DynSolValue::Tuple(vec![
        DynSolValue::FixedBytes(*PROOF_REQUEST_WITNESS_TYPE_HASH, 32),
        DynSolValue::Address(proof_commitment.signer),
        DynSolValue::Address(proof_commitment.market),
        DynSolValue::Uint(proof_commitment.nonce, 256),
        DynSolValue::Address(proof_commitment.rewardToken),
        DynSolValue::Uint(proof_commitment.maxRewardAmount, 256),
        DynSolValue::Uint(proof_commitment.minRewardAmount, 256),
        DynSolValue::Uint(U256::from(proof_commitment.minimumStake), 128),
        DynSolValue::Uint(U256::from(proof_commitment.startAuctionTimestamp), 64),
        DynSolValue::Uint(U256::from(proof_commitment.endAuctionTimestamp), 64),
        DynSolValue::Uint(U256::from(proof_commitment.provingTime), 32),
        DynSolValue::FixedBytes(proof_commitment.inputsCommitment, 32),
        DynSolValue::FixedBytes(extra_data_hash, 32),
    ]);

    // hash encoded witness
    let witness = keccak256(request_witness_values.abi_encode());

    // compute permit2 digest
    let token_permissions = TokenPermissions {
        token: proof_commitment.rewardToken,
        amount: proof_commitment.maxRewardAmount,
    };
    let token_permissions_bytes = token_permissions.abi_encode();

    let token_permissions_hash_preimage = [
        TOKEN_PERMISSIONS_TYPE_HASH.abi_encode(),
        token_permissions_bytes,
    ]
    .concat();

    // hash token permissions encoding
    let token_permissions_hash = keccak256(&token_permissions_hash_preimage);

    // encode data hash preimage
    let data_hash_preimage = DynSolValue::Tuple(vec![
        DynSolValue::FixedBytes(*REQUEST_PERMIT_TYPE_HASH, 32),
        DynSolValue::FixedBytes(token_permissions_hash, 32),
        DynSolValue::Address(proof_commitment.market),
        DynSolValue::Uint(proof_commitment.nonce, 256),
        DynSolValue::Uint(U256::from(proof_commitment.endAuctionTimestamp), 64),
        DynSolValue::FixedBytes(witness, 32),
    ])
    .abi_encode();

    // hash data hash encoding
    let data_hash = keccak256(&data_hash_preimage);

    // return the signable eip712 permit2 hash
    hash_typed_data(PERMIT2_DOMAIN_SEPARATOR, data_hash)
}
