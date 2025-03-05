use crate::abi::universal_bombetta::ProofRequestVerifierDetails;
use crate::abi::universal_porchetta::ProofOfferVerifierDetails;
use crate::systems::VerifierConstraints;
use alloy::primitives::{address, b256, keccak256, Address, B256};
use alloy::sol_types::SolValue;
use lazy_static::lazy_static;

// type strings
// string public constant FULL_PROOF_REQUEST_WITNESS_TYPE_STRING_STUB =
//   "ProofRequest witness)TokenPermissions(address token,uint256 amount)ProofRequest(address signer,address market,uint256 nonce,address token,uint256 maxRewardAmount,uint256 minRewardAmount,uint128 minimumStake,uint64 startAuctionTimestamp,uint64 endAuctionTimestamp,uint32 provingTime,bytes32 inputsCommitment,bytes extraData)";
// string public constant FULL_PROOF_OFFER_WITNESS_TYPE_STRING_STUB =
//   "ProofOffer witness)TokenPermissions(address token,uint256 amount)ProofOffer(address signer,address market,uint256 nonce,address rewardToken,uint256 rewardAmount,address stakeToken,uint256 stakeAmount,uint64 startAuctionTimestamp,uint64 endAuctionTimestamp,uint32 provingTime,bytes32 inputsCommitment,bytes extraData)";

// permit2
pub const PERMIT_TRANSFER_FROM_WITNESS_TYPEHASH_STUB: &str =
    "PermitWitnessTransferFrom(TokenPermissions permitted,address spender,uint256 nonce,uint256 deadline,";
pub const TOKEN_PERMISSIONS_TYPE_STRING: &str = "TokenPermissions(address token,uint256 amount)";
pub const PERMIT2_DOMAIN_SEPARATOR: B256 =
    b256!("2be86a484194028b8e9b1ac40deffff8868bf4ae32fd0a7db12030c6a18227e1");
pub const PERMIT2_ADDRESS: Address = address!("000000000022D473030F116dDEE9F6B43aC78BA3");

lazy_static! {
    pub static ref TOKEN_PERMISSIONS_TYPE_HASH: B256 =
        keccak256(TOKEN_PERMISSIONS_TYPE_STRING.as_bytes());
}

pub fn hash_typed_data(domain_separator: B256, data_hash: B256) -> B256 {
    let final_hash_preimage = [
        "\x19\x01".abi_encode_packed(),
        domain_separator.abi_encode(),
        data_hash.abi_encode(),
    ]
    .concat();

    keccak256(final_hash_preimage)
}

impl PartialEq<ProofRequestVerifierDetails> for VerifierConstraints {
    fn eq(&self, details: &ProofRequestVerifierDetails) -> bool {
        // Check each constraint only if it's specified
        self.verifier.is_none_or(|v| v == details.verifier)
            && self.selector.is_none_or(|s| s == details.selector)
            && self
                .is_sha_commitment
                .is_none_or(|sha| sha == details.isShaCommitment)
            && self.inputs_offset.is_none_or(|o| o == details.inputsOffset)
            && self.inputs_length.is_none_or(|l| l == details.inputsLength)
            && self
                .has_partial_commitment_result_check
                .is_none_or(|c| c == details.hasPartialCommitmentResultCheck)
            && self
                .submitted_partial_commitment_result_offset
                .is_none_or(|o| o == details.submittedPartialCommitmentResultOffset)
            && self
                .submitted_partial_commitment_result_length
                .is_none_or(|l| l == details.submittedPartialCommitmentResultLength)
            && self
                .predetermined_partial_commitment
                .is_none_or(|p| p == details.predeterminedPartialCommitment)
    }
}

impl PartialEq<ProofOfferVerifierDetails> for VerifierConstraints {
    fn eq(&self, details: &ProofOfferVerifierDetails) -> bool {
        // Check each constraint only if it's specified
        self.verifier.is_none_or(|v| v == details.verifier)
            && self.selector.is_none_or(|s| s == details.selector)
            && self
                .is_sha_commitment
                .is_none_or(|sha| sha == details.isShaCommitment)
            && self.inputs_offset.is_none_or(|o| o == details.inputsOffset)
            && self.inputs_length.is_none_or(|l| l == details.inputsLength)
    }
}
