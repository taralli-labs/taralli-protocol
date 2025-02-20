use std::{env, path::Path, str::FromStr, sync::Arc, time::Duration, u32};

use alloy::{
    primitives::{fixed_bytes, Address, FixedBytes, Uint, U256},
    providers::ProviderBuilder,
    signers::{k256::ecdsa, local::PrivateKeySigner, Signature, Signer},
    sol_types::SolValue,
};
use axum::{
    routing::{get, post},
    Router,
};
use dotenv::dotenv;
use rstest::*;

use taralli_primitives::alloy::primitives::PrimitiveSignature;
use taralli_primitives::systems::risc0::Risc0ProofParams;
use taralli_primitives::{
    market::UNIVERSAL_BOMBETTA_ADDRESS,
    systems::{ProvingSystemId, ProvingSystemParams},
    utils::{compute_permit2_digest, compute_request_witness},
    OnChainProofRequest, Request,
};
use taralli_server::{
    app_state::{AppState, AppStateConfig},
    config::Config,
    routes::{submit::submit_handler, subscribe::websocket_subscribe_handler},
    subscription_manager::SubscriptionManager,
};
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;

/// create dummy ECDSA signature
pub fn create_dummy_signature() -> PrimitiveSignature {
    PrimitiveSignature::try_from(&DUMMY_SIGNATURE_BYTES[..]).unwrap()
}

/// Dummy signature bytes used as placeholder before signing
pub const DUMMY_SIGNATURE_BYTES: [u8; 65] = [
    132, 12, 252, 87, 40, 69, 245, 120, 110, 112, 41, 132, 194, 165, 130, 82, 140, 173, 75, 73,
    178, 161, 11, 157, 177, 190, 127, 202, 144, 5, 133, 101, 37, 231, 16, 156, 235, 152, 22, 141,
    149, 176, 155, 24, 187, 246, 182, 133, 19, 14, 5, 98, 242, 51, 135, 125, 73, 43, 148, 238, 224,
    197, 182, 209, 0, // v value (false/0)
];

pub const DUMMY_PRIV_KEY: &str = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";

/// Generate a Request to be sent to the server.
/// The contents of the request are unimportant, as long as we pass the validation on submit().
#[fixture]
pub async fn request_fixture() -> Request<ProvingSystemParams> {
    let risc0_guest_program_path = Path::new(
        "/Users/gabrielsegatti/repo/taralli-protocol/contracts/test-proof-data/risc0/is-even",
    );

    println!("{:?}", risc0_guest_program_path);
    // proof input
    let proof_input = U256::from(1304);
    let inputs = proof_input.abi_encode();
    // load elf binary
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
            signer: Address::random(),
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
    // compute permit digest
    let permit2_digest = compute_permit2_digest(&proof_request.onchain_proof_request, witness);

    let signature = signer.sign_hash(&permit2_digest).await.expect("Couldn't sign req");
    proof_request.signature = signature;


    proof_request
}
