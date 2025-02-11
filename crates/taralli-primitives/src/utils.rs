use crate::abi::universal_bombetta::ISignatureTransfer::TokenPermissions;
use crate::abi::universal_bombetta::ProofRequestVerifierDetails;
use crate::abi::universal_bombetta::UniversalBombetta::ProofRequest;
use crate::abi::universal_porchetta::ProofOfferVerifierDetails;
use crate::abi::universal_porchetta::UniversalPorchetta::ProofOffer;
use crate::systems::VerifierConstraints;
use alloy::dyn_abi::DynSolValue;
use alloy::primitives::{address, b256, keccak256, Address, FixedBytes, B256, U256};
use alloy::signers::Signature;
use alloy::sol_types::SolValue;
use lazy_static::lazy_static;

/// type strings
// bombetta
pub const FULL_PROOF_REQUEST_WITNESS_TYPE_STRING_STUB: &str =
    "ProofRequest witness)TokenPermissions(address token,uint256 amount)ProofRequest(address signer,address market,uint256 nonce,address rewardToken,uint256 maxRewardAmount,uint256 minRewardAmount,uint128 minimumStake,uint64 startAuctionTimestamp,uint64 endAuctionTimestamp,uint32 provingTime,bytes32 inputsCommitment,bytes extraData)";
pub const PROOF_REQUEST_WITNESS_TYPE_STRING: &str =
    "ProofRequest(address signer,address market,uint256 nonce,address rewardToken,uint256 maxRewardAmount,uint256 minRewardAmount,uint128 minimumStake,uint64 startAuctionTimestamp,uint64 endAuctionTimestamp,uint32 provingTime,bytes32 inputsCommitment,bytes extraData)";
// porchetta
pub const FULL_PROOF_OFFER_WITNESS_TYPE_STRING_STUB: &str =
    "ProofOffer witness)TokenPermissions(address token,uint256 amount)ProofOffer(address signer,address market,uint256 nonce,address rewardToken,uint256 rewardAmount,address stakeToken,uint256 stakeAmount,uint64 startAuctionTimestamp,uint64 endAuctionTimestamp,uint32 provingTime,bytes32 inputsCommitment,bytes extraData)";
pub const PROOF_OFFER_WITNESS_TYPE_STRING: &str =
    "ProofOffer(address signer,address market,uint256 nonce,address rewardToken,uint256 rewardAmount,address stakeToken,uint256 stakeAmount,uint64 startAuctionTimestamp,uint64 endAuctionTimestamp,uint32 provingTime,bytes32 inputsCommitment,bytes extraData)";
// permit2
pub const PERMIT_TRANSFER_FROM_WITNESS_TYPEHASH_STUB: &str =
    "PermitWitnessTransferFrom(TokenPermissions permitted,address spender,uint256 nonce,uint256 deadline,";
pub const TOKEN_PERMISSIONS_TYPE_STRING: &str = "TokenPermissions(address token,uint256 amount)";
pub const PERMIT2_DOMAIN_SEPARATOR: B256 =
    b256!("2be86a484194028b8e9b1ac40deffff8868bf4ae32fd0a7db12030c6a18227e1");
pub const PERMIT2_ADDRESS: Address = address!("000000000022D473030F116dDEE9F6B43aC78BA3");

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
    pub static ref TOKEN_PERMISSIONS_TYPE_HASH: B256 =
        keccak256(TOKEN_PERMISSIONS_TYPE_STRING.as_bytes());
    pub static ref PROOF_REQUEST_WITNESS_TYPE_HASH: B256 =
        keccak256(PROOF_REQUEST_WITNESS_TYPE_STRING.as_bytes());
    pub static ref PROOF_OFFER_WITNESS_TYPE_HASH: B256 =
        keccak256(PROOF_OFFER_WITNESS_TYPE_STRING.as_bytes());
}

pub fn compute_request_id(proof_request: &ProofRequest, signature: Signature) -> B256 {
    // encode + hash `extraData` and `signature`
    let extra_data_hash = keccak256(proof_request.extraData.abi_encode());
    let signature_hash = keccak256(signature.as_bytes().abi_encode());

    // Encode OnChainProofRequest + Signature
    let values = DynSolValue::Tuple(vec![
        DynSolValue::Address(proof_request.market),
        DynSolValue::Uint(proof_request.nonce, 256),
        DynSolValue::Address(proof_request.rewardToken),
        DynSolValue::Uint(proof_request.maxRewardAmount, 256),
        DynSolValue::Uint(proof_request.minRewardAmount, 256),
        DynSolValue::Uint(U256::from(proof_request.minimumStake), 128),
        DynSolValue::Uint(U256::from(proof_request.startAuctionTimestamp), 64),
        DynSolValue::Uint(U256::from(proof_request.endAuctionTimestamp), 64),
        DynSolValue::Uint(U256::from(proof_request.provingTime), 32),
        DynSolValue::FixedBytes(proof_request.inputsCommitment, 32),
        DynSolValue::FixedBytes(extra_data_hash, 32),
        DynSolValue::FixedBytes(signature_hash, 32),
    ]);
    let preimage = values.abi_encode();

    // hash encoded preimage to get request id
    keccak256(&preimage)
}

pub fn compute_request_witness(proof_request: &ProofRequest) -> FixedBytes<32> {
    // encode witness data
    let extra_data_hash = keccak256(&proof_request.extraData);
    let request_witness_values = DynSolValue::Tuple(vec![
        DynSolValue::FixedBytes(*PROOF_REQUEST_WITNESS_TYPE_HASH, 32),
        DynSolValue::Address(proof_request.signer),
        DynSolValue::Address(proof_request.market),
        DynSolValue::Uint(proof_request.nonce, 256),
        DynSolValue::Address(proof_request.rewardToken),
        DynSolValue::Uint(proof_request.maxRewardAmount, 256),
        DynSolValue::Uint(proof_request.minRewardAmount, 256),
        DynSolValue::Uint(U256::from(proof_request.minimumStake), 128),
        DynSolValue::Uint(U256::from(proof_request.startAuctionTimestamp), 64),
        DynSolValue::Uint(U256::from(proof_request.endAuctionTimestamp), 64),
        DynSolValue::Uint(U256::from(proof_request.provingTime), 32),
        DynSolValue::FixedBytes(proof_request.inputsCommitment, 32),
        DynSolValue::FixedBytes(extra_data_hash, 32),
    ]);

    // hash encoded witness
    keccak256(request_witness_values.abi_encode())
}

pub fn compute_request_permit2_digest(proof_request: &ProofRequest, witness: B256) -> B256 {
    // encode token permissions data
    let token_permissions = TokenPermissions {
        token: proof_request.rewardToken,
        amount: proof_request.maxRewardAmount,
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
        DynSolValue::Address(proof_request.market),
        DynSolValue::Uint(proof_request.nonce, 256),
        DynSolValue::Uint(U256::from(proof_request.endAuctionTimestamp), 64),
        DynSolValue::FixedBytes(witness, 32),
    ])
    .abi_encode();

    // hash data hash encoding
    let data_hash = keccak256(&data_hash_preimage);

    // return the signable eip712 permit2 hash
    hash_typed_data(PERMIT2_DOMAIN_SEPARATOR, data_hash)
}

pub fn compute_offer_id(proof_offer: &ProofOffer, signature: Signature) -> B256 {
    // encode + hash `extraData` and `signature`
    let extra_data_hash = keccak256(proof_offer.extraData.abi_encode());
    let signature_hash = keccak256(signature.as_bytes().abi_encode());

    // Encode ProofRequest + Signature
    let values = DynSolValue::Tuple(vec![
        DynSolValue::Address(proof_offer.market),
        DynSolValue::Uint(proof_offer.nonce, 256),
        DynSolValue::Address(proof_offer.rewardToken),
        DynSolValue::Uint(proof_offer.rewardAmount, 256),
        DynSolValue::Address(proof_offer.stakeToken),
        DynSolValue::Uint(proof_offer.stakeAmount, 256),
        DynSolValue::Uint(U256::from(proof_offer.startAuctionTimestamp), 64),
        DynSolValue::Uint(U256::from(proof_offer.endAuctionTimestamp), 64),
        DynSolValue::Uint(U256::from(proof_offer.provingTime), 32),
        DynSolValue::FixedBytes(proof_offer.inputsCommitment, 32),
        DynSolValue::FixedBytes(extra_data_hash, 32),
        DynSolValue::FixedBytes(signature_hash, 32),
    ]);
    let preimage = values.abi_encode();

    // hash encoded preimage to get request id
    keccak256(&preimage)
}

pub fn compute_offer_witness(proof_offer: &ProofOffer) -> FixedBytes<32> {
    // encode witness data
    let extra_data_hash = keccak256(&proof_offer.extraData);
    let request_witness_values = DynSolValue::Tuple(vec![
        DynSolValue::FixedBytes(*PROOF_OFFER_WITNESS_TYPE_HASH, 32),
        DynSolValue::Address(proof_offer.signer),
        DynSolValue::Address(proof_offer.market),
        DynSolValue::Uint(proof_offer.nonce, 256),
        DynSolValue::Address(proof_offer.rewardToken),
        DynSolValue::Uint(proof_offer.rewardAmount, 256),
        DynSolValue::Address(proof_offer.stakeToken),
        DynSolValue::Uint(U256::from(proof_offer.stakeAmount), 128),
        DynSolValue::Uint(U256::from(proof_offer.startAuctionTimestamp), 64),
        DynSolValue::Uint(U256::from(proof_offer.endAuctionTimestamp), 64),
        DynSolValue::Uint(U256::from(proof_offer.provingTime), 32),
        DynSolValue::FixedBytes(proof_offer.inputsCommitment, 32),
        DynSolValue::FixedBytes(extra_data_hash, 32),
    ]);

    // hash encoded witness
    keccak256(request_witness_values.abi_encode())
}

pub fn compute_offer_permit2_digest(proof_offer: &ProofOffer, witness: B256) -> B256 {
    // encode token permissions data
    let token_permissions = TokenPermissions {
        token: proof_offer.stakeToken,
        amount: proof_offer.stakeAmount,
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
        DynSolValue::Address(proof_offer.market),
        DynSolValue::Uint(proof_offer.nonce, 256),
        DynSolValue::Uint(U256::from(proof_offer.endAuctionTimestamp), 64),
        DynSolValue::FixedBytes(witness, 32),
    ])
    .abi_encode();

    // hash data hash encoding
    let data_hash = keccak256(&data_hash_preimage);

    // return the signable eip712 permit2 hash
    hash_typed_data(PERMIT2_DOMAIN_SEPARATOR, data_hash)
}

fn hash_typed_data(domain_separator: B256, data_hash: B256) -> B256 {
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

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::primitives::Bytes;
    use tokio;

    /// Dummy signature bytes used for testing and placeholder signatures
    pub const DUMMY_SIGNATURE_BYTES: [u8; 65] = [
        132, 12, 252, 87, 40, 69, 245, 120, 110, 112, 41, 132, 194, 165, 130, 82, 140, 173, 75, 73,
        178, 161, 11, 157, 177, 190, 127, 202, 144, 5, 133, 101, 37, 231, 16, 156, 235, 152, 22,
        141, 149, 176, 155, 24, 187, 246, 182, 133, 19, 14, 5, 98, 242, 51, 135, 125, 73, 43, 148,
        238, 224, 197, 182, 209, 0, // v value (false/0)
    ];

    // Mock setup function to generate sample OnChainProofRequest and other inputs
    fn get_mock_proof_request() -> ProofRequest {
        ProofRequest {
            signer: address!("0000000000000000000000000000000000000003"),
            market: address!("0000000000000000000000000000000000000003"),
            nonce: U256::ZERO,
            rewardToken: address!("0000000000000000000000000000000000000003"),
            maxRewardAmount: U256::ZERO,
            minRewardAmount: U256::ZERO,
            minimumStake: 0,
            startAuctionTimestamp: 0,
            endAuctionTimestamp: 0,
            provingTime: 0,
            inputsCommitment: b256!(
                "0000000000000000000000000000000000000000000000000000000000000000"
            ),
            extraData: Bytes::from(""),
        }
    }

    #[tokio::test]
    async fn test_compute_request_id() {
        let mock_request = get_mock_proof_request();
        let signature = Signature::try_from(&DUMMY_SIGNATURE_BYTES[..]).unwrap();
        let local_result = compute_request_id(&mock_request, signature);
        println!("Local result: {:?}", local_result);
    }

    #[tokio::test]
    async fn test_compute_request_witness() {
        let mock_request = get_mock_proof_request();
        let local_result = compute_request_witness(&mock_request);
        println!("Local result: {:?}", local_result);
    }

    #[tokio::test]
    async fn test_compute_permit2_digest() {
        let mock_request = get_mock_proof_request();
        let witness = b256!("e6a6cf5ad10b2e60506ffc96bf4d74f8853c100ded900069fc5dc42faa55c1fa");
        let local_result = compute_request_permit2_digest(&mock_request, witness);
        println!("local digest: {}", local_result);
    }
}
