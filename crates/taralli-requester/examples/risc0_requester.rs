use alloy::dyn_abi::DynSolValue;
use alloy::primitives::{address, fixed_bytes, Bytes, FixedBytes, B256, U256};
use alloy::providers::ProviderBuilder;
use alloy::signers::k256::sha2::Sha256;
use alloy::signers::local::PrivateKeySigner;
use alloy::sol_types::SolValue;
use color_eyre::Result;
use dotenv::dotenv;
use sha3::Digest;
use std::env;
use std::path::Path;
use std::str::FromStr;
use taralli_primitives::abi::universal_bombetta::VerifierDetails;
use taralli_primitives::market::UNIVERSAL_BOMBETTA_ADDRESS;
use taralli_primitives::systems::risc0::Risc0ProofParams;
use taralli_primitives::systems::ProvingSystemId;
use taralli_requester::config::RequesterConfig;
use taralli_requester::RequesterClient;
use tracing::Level;
use tracing_subscriber::EnvFilter;
use url::Url;

/// This is an example workflow a proof request for Risc0 using the taralli protocol:
/// 1. Read in needed state variables, define rpc provider and define signer
/// 2. Set up the requester config -- constant values that do not change with each request
/// 3. Set up the request builder with default values
/// 4. load in concrete information for a specific proof request.
///    (reward params, proof data inputs, verification commitment data, etc.)
/// 5. Extend and finish building the request from default builder to build a specific proof request
///    using specific inputs
/// 6. sign the fully built request.
/// 7. submit the signed request to the server and start tracking its status
#[tokio::main]
async fn main() -> Result<()> {
    // setup tracing for client execution
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_max_level(Level::INFO)
        .init();

    // Load environment variables from the `.env` file
    dotenv().ok();
    let server_url = Url::parse(&env::var("SERVER_URL")?)?; // local server instance
    let rpc_url = Url::parse(&env::var("RPC_URL")?)?; // Holesky testnet
    let priv_key = &env::var("REQUESTER_PRIVATE_KEY")?; // Holesky private key

    // proving system information data
    let risc0_guest_program_path = Path::new("./contracts/test-proof-data/risc0/is-even");
    let risc0_image_id: FixedBytes<32> =
        fixed_bytes!("cb7d04f8807ec1b6ffa79c29e4b7c6cb071c1bcc1de2e6c6068882a55ad8f3a8");

    // proof input
    let proof_input = U256::from(1304);
    let inputs = proof_input.abi_encode();
    // load elf binary
    let elf = std::fs::read(risc0_guest_program_path)?;

    // on chain proof request data
    let market_address = UNIVERSAL_BOMBETTA_ADDRESS;
    let reward_token_address = address!("89fF1B147026815cf497AA45D4FDc2DF51Ed7f00");
    let reward_token_decimals = 18u8;
    let max_reward_amount = U256::from(100e18); // 100 tokens
    let min_reward_amount = U256::from(10); // 10 wei of tokens
    let minimum_stake = 1; // 1 wei, for testing
    let proving_time = 60u32; // 1 min
    let auction_length = 60u32; // 1 min
    let verifier_address = address!("31766974fb795dF3f7d0c010a3D5c55e4bd8113e");
    let verify_function_selector: FixedBytes<4> = fixed_bytes!("ab750e75");
    let public_inputs_offset = U256::from(32);
    let public_inputs_length = U256::from(64);
    let is_sha_commitment = true;
    let has_partial_commitment_result_check = false;
    let submitted_partial_commitment_result_offset = U256::from(0);
    let submitted_partial_commitment_result_length = U256::from(0);
    let pre_determined_partial_commitment: FixedBytes<32> =
        fixed_bytes!("0000000000000000000000000000000000000000000000000000000000000000");

    // signer
    let signer = PrivateKeySigner::from_str(priv_key)?;

    // build provider
    let rpc_provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .on_http(rpc_url);

    // build requester config
    let config = RequesterConfig::new(
        rpc_provider.clone(),
        signer,
        server_url,
        market_address,
        reward_token_address,
        reward_token_decimals,
        ProvingSystemId::Risc0,
    );

    // instantiate requester client
    let requester = RequesterClient::new(config);

    // set builder defaults
    let builder_default = requester
        .builder
        .clone()
        .set_auction_length(auction_length) // 30 secs
        .reward_token_address(reward_token_address);

    // builder that extends from default builder
    let builder = builder_default.clone();

    // proving system information
    let proof_info = serde_json::to_value(Risc0ProofParams { elf, inputs })?;

    // load verification commitments
    // abi encode image id and proof input hash
    let public_inputs_commitment_preimage = DynSolValue::Tuple(vec![
        DynSolValue::FixedBytes(risc0_image_id, 32),
        DynSolValue::Uint(proof_input, 256),
    ]);
    let public_inputs_commitment_digest =
        Sha256::digest(public_inputs_commitment_preimage.abi_encode());
    let public_inputs_commitment = B256::from_slice(public_inputs_commitment_digest.as_slice());

    // build verifier details using external tool
    let verifier_details = VerifierDetails {
        verifier: verifier_address,
        selector: verify_function_selector,
        isShaCommitment: is_sha_commitment,
        publicInputsOffset: public_inputs_offset,
        publicInputsLength: public_inputs_length,
        hasPartialCommitmentResultCheck: has_partial_commitment_result_check,
        submittedPartialCommitmentResultOffset: submitted_partial_commitment_result_offset,
        submittedPartialCommitmentResultLength: submitted_partial_commitment_result_length,
        predeterminedPartialCommitment: pre_determined_partial_commitment,
    };
    // set extra_data = abi encoded verifier details
    let extra_data = Bytes::from(VerifierDetails::abi_encode(&verifier_details));

    // finish building proof request
    let proof_request = builder
        .set_new_nonce()
        .await?
        .set_reward_params(minimum_stake, min_reward_amount, max_reward_amount)
        .proving_time(proving_time)
        .proving_system_information(proof_info)
        .set_verification_commitment_params(public_inputs_commitment, extra_data)
        .set_auction_timestamps_from_auction_length()
        .await?
        .build(); // convert RequestBuilder into ProofRequest

    // sign built request
    let signed_request = requester.sign_request(proof_request.clone()).await?;

    // validate before submitting
    requester.validate_request(&signed_request)?;

    // TODO: Add a retry policy
    requester
        .submit_and_track_request(signed_request, auction_length as u64)
        .await?;

    Ok(())
}
