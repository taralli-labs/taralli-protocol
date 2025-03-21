use std::{path::Path, str::FromStr, u32};

use alloy::{
    primitives::{address, Address, FixedBytes, Uint, U256, PrimitiveSignature},
    signers::{local::PrivateKeySigner, Signer},
    sol_types::SolValue,
};
use rstest::*;

use taralli_client::api::{submit::SubmitApiClient, subscribe::SubscribeApiClient};
use taralli_primitives::{abi::universal_bombetta::UniversalBombetta::ProofRequest, intents::ComputeIntent, systems::{risc0::Risc0ProofParams, SystemIdMask}};
use taralli_primitives::{
    markets::SEPOLIA_UNIVERSAL_BOMBETTA_ADDRESS,
    systems::{SystemId, SystemParams},
    intents::request::ComputeRequest
};
use url::Url;

const DUMMY_PRIV_KEY: &str = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";

/// signature bytes used as placeholder before signing
pub const MOCK_SIGNATURE_BYTES: [u8; 65] = [
    132, 12, 252, 87, 40, 69, 245, 120, 110, 112, 41, 132, 194, 165, 130, 82, 140, 173, 75, 73,
    178, 161, 11, 157, 177, 190, 127, 202, 144, 5, 133, 101, 37, 231, 16, 156, 235, 152, 22, 141,
    149, 176, 155, 24, 187, 246, 182, 133, 19, 14, 5, 98, 242, 51, 135, 125, 73, 43, 148, 238, 224,
    197, 182, 209, 0,
];

#[fixture]
pub fn requester_fixture() -> SubmitApiClient {
    SubmitApiClient::new(Url::parse("http://localhost:8080").unwrap())
}

#[fixture]
pub fn provider_fixture() -> SubscribeApiClient {
    SubscribeApiClient::new(Url::parse("http://localhost:8080").unwrap(), SystemIdMask::MAX)
}

/// Generate a Request to be sent to the server.
/// The contents of the request are unimportant, as long as we pass the validation on submit().
#[fixture]
pub async fn request_fixture() -> ComputeRequest<SystemParams> {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("Failed to get crates")
        .parent()
        .expect("Failed to get root");
    let risc0_guest_program_path = repo_root.join("contracts/test-proof-data/risc0/is-even");

    let proof_input = U256::from(1304);
    let inputs = proof_input.abi_encode();
    let elf = std::fs::read(risc0_guest_program_path).expect("Couldn't read elf");
    let mut compute_request: ComputeRequest<SystemParams> = ComputeRequest {
        system_id: SystemId::Risc0,
        system: SystemParams::try_from((
            &SystemId::Risc0,
            serde_json::to_value(Risc0ProofParams { elf, inputs })
                .unwrap()
                .to_string()
                .into_bytes(),
        ))
        .unwrap(),
        proof_request: ProofRequest {
            signer: address!("f39Fd6e51aad88F6F4ce6aB8827279cffFb92266"),
            market: SEPOLIA_UNIVERSAL_BOMBETTA_ADDRESS,
            nonce: Uint::from(0u64),
            rewardToken: Address::random(),
            maxRewardAmount: U256::from(0),
            minRewardAmount: U256::from(0),
            minimumStake: 0,
            startAuctionTimestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            endAuctionTimestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
                + 86400, // 1 day in the future
            provingTime: u32::MAX,
            inputsCommitment: FixedBytes::<32>::new([0u8; 32]),
            extraData: vec![].into(),
        },
        signature: signature_fixture(),
    };

    let signer = PrivateKeySigner::from_str(DUMMY_PRIV_KEY).expect("Couldn't get priv key");
    let permit2_digest = compute_request.compute_permit2_digest();
    let signature = signer
        .sign_hash(&permit2_digest)
        .await
        .expect("Couldn't sign req");
    compute_request.signature = signature;

    compute_request
}

/// create dummy ECDSA signature
pub fn signature_fixture() -> PrimitiveSignature {
    PrimitiveSignature::try_from(&MOCK_SIGNATURE_BYTES[..])
        .expect("Unreachable: Mock Signature try from failure")
}