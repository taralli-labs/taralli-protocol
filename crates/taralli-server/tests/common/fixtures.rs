use std::{fs::File, path::Path, str::FromStr};

use alloy::{
    primitives::{address, Address, FixedBytes, PrimitiveSignature, Uint, U256},
    providers::ProviderBuilder,
    signers::{local::PrivateKeySigner, Signer},
    sol_types::SolValue,
};
use axum::{
    routing::{get, post},
    Router,
};
use futures::FutureExt;
use rstest::*;

use serde_json::Value;
use std::sync::Arc;
use taralli_client::api::{submit::SubmitApiClient, subscribe::SubscribeApiClient};
use taralli_primitives::{
    abi::universal_bombetta::UniversalBombetta::ProofRequest,
    intents::ComputeIntent,
    systems::{arkworks::ArkworksProofParams, risc0::Risc0ProofParams, ALL_PROVING_SYSTEMS},
};
use taralli_primitives::{
    intents::request::ComputeRequest,
    markets::SEPOLIA_UNIVERSAL_BOMBETTA_ADDRESS,
    systems::{SystemId, SystemParams},
};
use taralli_server::{
    config::{Markets, ServerValidationConfigs},
    routes::{submit::submit_request_handler, subscribe::websocket_subscribe_handler},
    state::{request::RequestState, BaseState},
    subscription_manager::{self, SubscriptionManager},
};
use tower_http::trace::TraceLayer;
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
    SubscribeApiClient::new(
        Url::parse("http://localhost:8080").unwrap(),
        *ALL_PROVING_SYSTEMS,
    )
}

/// Generate a Request to be sent to the server.
/// The contents of the request are unimportant, as long as we pass the validation on submit().
#[fixture]
pub fn risc0_request_fixture() -> ComputeRequest<SystemParams> {
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
            nonce: U256::from(0u64),
            rewardToken: Address::ZERO,
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
        .now_or_never()
        .expect("Couldn't sign req async")
        .expect("Couldn't sign req");
    compute_request.signature = signature;

    compute_request
}

#[fixture]
/// Also generates a Request to be sent to the server.
/// This one, however, is much bigger than the risc0 above.
pub fn groth16_request_fixture() -> ComputeRequest<SystemParams> {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("Failed to get crates")
        .parent()
        .expect("Failed to get root");
    let r1cs_guest_program_path =
        repo_root.join("contracts/test-proof-data/groth16/sha/sha256_test512.r1cs");
    let wasm_path = repo_root
        .join("contracts/test-proof-data/groth16/sha/sha256_test512_js/sha256_test512.wasm");
    let input = repo_root.join("contracts/test-proof-data/groth16/sha/input.json");

    let r1cs = std::fs::read(r1cs_guest_program_path).expect("Couldn't read r1cs");
    let wasm = std::fs::read(wasm_path).expect("Couldn't read wasm");
    let inputs: Value =
        serde_json::from_reader(File::open(input).expect("Couldn't open input file"))
            .expect("Couldn't read input file");
    let mut proof_request: ComputeRequest<SystemParams> = ComputeRequest {
        system_id: SystemId::Arkworks,
        system: SystemParams::try_from((
            &SystemId::Arkworks,
            serde_json::to_value(ArkworksProofParams { r1cs, wasm, inputs })
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
    let permit2_digest = proof_request.compute_permit2_digest();
    let signature = signer
        .sign_hash(&permit2_digest)
        .now_or_never()
        .expect("Couldn't sign req async")
        .expect("Couldn't sign req");
    proof_request.signature = signature;

    proof_request
}

/// create dummy ECDSA signature
#[must_use]
pub fn signature_fixture() -> PrimitiveSignature {
    PrimitiveSignature::try_from(&MOCK_SIGNATURE_BYTES[..])
        .expect("Unreachable: Mock Signature try from failure")
}

#[fixture]
/// We create a new instance of the server, with the subscription manager.
/// The reasoning here is that we want to test how the client/provider handles receiving corrupted/wrong data.
/// We can't just submit it to the server directly via requester, because the server would reject it.
/// So we instantiate a server with a subscription manager, which we then use in our tests, since the subscription manager is the step before sending data through providers.
/// For more information, check `subscribe.rs`.
pub fn setup_app() -> (Router, Arc<SubscriptionManager>) {
    let rpc_provider =
        ProviderBuilder::new().on_http(reqwest::Url::parse("http://localhost:8080").unwrap());
    let subscription_manager: Arc<SubscriptionManager> =
        subscription_manager::SubscriptionManager::new(2).into();

    let base_state = BaseState::new(
        rpc_provider.clone(),
        Markets {
            universal_bombetta: SEPOLIA_UNIVERSAL_BOMBETTA_ADDRESS,
            universal_porchetta: Address::random(),
        },
        std::time::Duration::from_secs(10),
        ServerValidationConfigs {
            request: Default::default(),
            offer: Default::default(),
        },
    );
    let request_state = RequestState::new(base_state.clone(), subscription_manager.clone());

    (
        Router::new()
            .route("/submit", post(submit_request_handler))
            .route("/subscribe", get(websocket_subscribe_handler))
            .with_state(request_state)
            .layer(TraceLayer::new_for_http()),
        subscription_manager,
    )
}
