use std::{path::Path, str::FromStr, u32};

use alloy::{
    primitives::{address, Address, FixedBytes, Uint, U256},
    signers::{local::PrivateKeySigner, Signer},
    sol_types::SolValue,
};
use rstest::*;

use taralli_primitives::systems::risc0::Risc0ProofParams;
use taralli_primitives::{
    markets::UNIVERSAL_BOMBETTA_ADDRESS,
    systems::{SystemId, SystemParams},
    utils::{compute_permit2_digest, compute_request_witness},
    ProofRequest, ComputeRequest,
};
use taralli_provider::{api::ProviderApi, config::ApiConfig};
use taralli_requester::{api::RequesterApi, create_dummy_signature};
use url::Url;

const DUMMY_PRIV_KEY: &str = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";

#[fixture]
pub fn requester_fixture() -> RequesterApi {
    RequesterApi::new(Url::parse("http://localhost:8000").unwrap())
}

#[fixture]
pub fn provider_fixture() -> ProviderApi {
    ProviderApi::new(ApiConfig::default())
}

/// Generate a Request to be sent to the server.
/// The contents of the request are unimportant, as long as we pass the validation on submit().
#[fixture]
pub async fn request_fixture() -> Request<ProvingSystemParams> {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("Failed to get crates")
        .parent()
        .expect("Failed to get root");
    let risc0_guest_program_path = repo_root.join("contracts/test-proof-data/risc0/is-even");

    let proof_input = U256::from(1304);
    let inputs = proof_input.abi_encode();
    let elf = std::fs::read(risc0_guest_program_path).expect("Couldn't read elf");
    let mut proof_request: Request<ProvingSystemParams> = Request {
        proving_system_id: ProvingSystemId::Risc0,
        proving_system_information: ProvingSystemParams::try_from((
            &ProvingSystemId::Risc0,
            serde_json::to_value(Risc0ProofParams { elf, inputs })
                .unwrap()
                .to_string()
                .into_bytes(),
        ))
        .unwrap(),
        onchain_proof_request: OnChainProofRequest {
            signer: address!("f39Fd6e51aad88F6F4ce6aB8827279cffFb92266"),
            market: UNIVERSAL_BOMBETTA_ADDRESS,
            nonce: Uint::from(0u64),
            token: Address::random(),
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
            publicInputsCommitment: FixedBytes::<32>::new([0u8; 32]),
            extraData: vec![].into(),
        },
        signature: create_dummy_signature(),
    };

    let signer = PrivateKeySigner::from_str(DUMMY_PRIV_KEY).expect("Couldn't get priv key");
    let witness = compute_request_witness(&proof_request.onchain_proof_request);
    let permit2_digest = compute_permit2_digest(&proof_request.onchain_proof_request, witness);
    let signature = signer
        .sign_hash(&permit2_digest)
        .await
        .expect("Couldn't sign req");
    proof_request.signature = signature;

    proof_request
}