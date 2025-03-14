use alloy::{
    dyn_abi::DynSolValue,
    primitives::{keccak256, FixedBytes, PrimitiveSignature, B256, U256},
    sol_types::SolValue,
};
use serde::{Deserialize, Serialize};

use crate::{
    abi::{permit2::ISignatureTransfer::TokenPermissions, universal_porchetta::UniversalPorchetta},
    systems::{System, SystemId},
    utils::{
        hash_typed_data, PERMIT2_DOMAIN_SEPARATOR, PERMIT_TRANSFER_FROM_WITNESS_TYPEHASH_STUB,
        TOKEN_PERMISSIONS_TYPE_HASH,
    },
};
use lazy_static::lazy_static;

use super::ComputeIntent;

/// porchetta signature constants
pub const FULL_PROOF_OFFER_WITNESS_TYPE_STRING_STUB: &str =
    "ProofOffer witness)TokenPermissions(address token,uint256 amount)ProofOffer(address signer,address market,uint256 nonce,address rewardToken,uint256 rewardAmount,address stakeToken,uint256 stakeAmount,uint64 startAuctionTimestamp,uint64 endAuctionTimestamp,uint32 provingTime,bytes32 inputsCommitment,bytes extraData)";
pub const PROOF_OFFER_WITNESS_TYPE_STRING: &str =
    "ProofOffer(address signer,address market,uint256 nonce,address rewardToken,uint256 rewardAmount,address stakeToken,uint256 stakeAmount,uint64 startAuctionTimestamp,uint64 endAuctionTimestamp,uint32 provingTime,bytes32 inputsCommitment,bytes extraData)";

lazy_static! {
    pub static ref OFFER_PERMIT_TYPE_HASH: B256 = {
        // craft preimage
        let type_hash_preimage = [
            PERMIT_TRANSFER_FROM_WITNESS_TYPEHASH_STUB.as_bytes(),
            FULL_PROOF_OFFER_WITNESS_TYPE_STRING_STUB.as_bytes(),
        ]
        .concat();
        // Compute hash
        keccak256(&type_hash_preimage)
    };
    pub static ref PROOF_OFFER_WITNESS_TYPE_HASH: B256 =
        keccak256(PROOF_OFFER_WITNESS_TYPE_STRING.as_bytes());
}

/// Compute offer type generic over all Systems
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(bound = "S: System")]
pub struct ComputeOffer<S: System> {
    pub system_id: SystemId,
    pub system: S,
    pub proof_offer: UniversalPorchetta::ProofOffer,
    pub signature: PrimitiveSignature,
}

/// Generic compute offer implementation
impl<S: System> ComputeIntent for ComputeOffer<S> {
    type System = S;
    type ProofCommitment = UniversalPorchetta::ProofOffer;

    fn type_string(&self) -> String {
        "offer".to_string()
    }

    fn compute_id(&self) -> FixedBytes<32> {
        // encode + hash `extraData` and `signature`
        let extra_data_hash = keccak256(self.proof_offer.extraData.abi_encode());
        let signature_hash = keccak256(self.signature.as_bytes().abi_encode());

        // Encode ProofOffer + Signature
        let values = DynSolValue::Tuple(vec![
            DynSolValue::Address(self.proof_offer.signer),
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

        // hash encoded preimage to get intent id
        keccak256(&preimage)
    }

    fn compute_permit2_digest(&self) -> FixedBytes<32> {
        // compute witness
        let extra_data_hash = keccak256(&self.proof_offer.extraData);
        let offer_witness_values = DynSolValue::Tuple(vec![
            DynSolValue::FixedBytes(*PROOF_OFFER_WITNESS_TYPE_HASH, 32),
            DynSolValue::Address(self.proof_offer.signer),
            DynSolValue::Address(self.proof_offer.market),
            DynSolValue::Uint(self.proof_offer.nonce, 256),
            DynSolValue::Address(self.proof_offer.rewardToken),
            DynSolValue::Uint(self.proof_offer.rewardAmount, 256),
            DynSolValue::Address(self.proof_offer.stakeToken),
            DynSolValue::Uint(U256::from(self.proof_offer.stakeAmount), 128),
            DynSolValue::Uint(U256::from(self.proof_offer.startAuctionTimestamp), 64),
            DynSolValue::Uint(U256::from(self.proof_offer.endAuctionTimestamp), 64),
            DynSolValue::Uint(U256::from(self.proof_offer.provingTime), 32),
            DynSolValue::FixedBytes(self.proof_offer.inputsCommitment, 32),
            DynSolValue::FixedBytes(extra_data_hash, 32),
        ]);

        // hash encoded witness
        let witness = keccak256(offer_witness_values.abi_encode());

        // encode token permissions data
        let token_permissions = TokenPermissions {
            token: self.proof_offer.stakeToken,
            amount: self.proof_offer.stakeAmount,
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
            DynSolValue::FixedBytes(*OFFER_PERMIT_TYPE_HASH, 32),
            DynSolValue::FixedBytes(token_permissions_hash, 32),
            DynSolValue::Address(self.proof_offer.market),
            DynSolValue::Uint(self.proof_offer.nonce, 256),
            DynSolValue::Uint(U256::from(self.proof_offer.endAuctionTimestamp), 64),
            DynSolValue::FixedBytes(witness, 32),
        ])
        .abi_encode();

        // hash data hash encoding
        let data_hash = keccak256(&data_hash_preimage);

        // return the signable eip712 permit2 hash
        hash_typed_data(PERMIT2_DOMAIN_SEPARATOR, data_hash)
    }
}
