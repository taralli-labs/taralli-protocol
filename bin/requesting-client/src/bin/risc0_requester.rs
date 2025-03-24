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
use taralli_client::client::requester::requesting::RequesterRequestingClient;
use taralli_client::intent_builder::IntentBuilder;
use taralli_primitives::abi::universal_bombetta::VerifierDetails;
use taralli_primitives::markets::{Network, SEPOLIA_UNIVERSAL_BOMBETTA_ADDRESS};
use taralli_primitives::systems::risc0::{Risc0ProofParams, Risc0VerifierConstraints};
use taralli_primitives::systems::SystemId;
use taralli_primitives::validation::request::RequestValidationConfig;
use taralli_primitives::validation::BaseValidationConfig;
use tracing::Level;
use tracing_subscriber::EnvFilter;
use url::Url;

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
    let rpc_url = Url::parse(&env::var("RPC_URL")?)?; // testnet
    let priv_key = &env::var("REQUESTER_PRIVATE_KEY")?; // private key

    // system workload data
    let risc0_guest_program_path = Path::new("./contracts/test-proof-data/risc0/is-even");
    let risc0_image_id: FixedBytes<32> =
        fixed_bytes!("cb7d04f8807ec1b6ffa79c29e4b7c6cb071c1bcc1de2e6c6068882a55ad8f3a8");

    // input data
    let proof_input = U256::from(1304);
    let inputs = proof_input.abi_encode();
    // load elf binary
    let elf = std::fs::read(risc0_guest_program_path)?;

    // proof commitment data
    let reward_token_address = address!("b54061f59AcF94f86ee414C9a220aFFE8BbE6B35");
    let reward_token_decimals = 18u8;
    let max_reward_amount = U256::from(100e18); // 100 tokens
    let min_reward_amount = U256::from(10); // 10 wei of tokens
    let minimum_stake = 1; // 1 wei, for testing
    let proving_time = 60u32; // 1 min
    let auction_length = 60u32; // 1 min
                                // Risc0 sepolia groth16 verifier
    let verifier_address = address!("AC292cF957Dd5BA174cdA13b05C16aFC71700327");
    // verify(bytes calldata seal, bytes32 imageId, bytes32 journalDigest)
    let verify_function_selector: FixedBytes<4> = fixed_bytes!("ab750e75");
    // offset and length to extract inputs field
    let inputs_offset = U256::from(32);
    let inputs_length = U256::from(64);
    // uses sha
    let is_sha_commitment = true;
    // no partial commitments used
    let has_partial_commitment_result_check = false;
    let submitted_partial_commitment_result_offset = U256::from(0);
    let submitted_partial_commitment_result_length = U256::from(0);
    let pre_determined_partial_commitment: FixedBytes<32> =
        fixed_bytes!("0000000000000000000000000000000000000000000000000000000000000000");

    // network
    let network = Network::Sepolia;

    // signer
    let signer = PrivateKeySigner::from_str(priv_key)?;

    // build rpc provider
    let rpc_provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .on_http(rpc_url);

    // validation config to check requests are correct
    let validation_config = RequestValidationConfig {
        base: BaseValidationConfig::default(),
        maximum_allowed_stake: 10000000000000000000, // 10 ether
    };

    // instantiate requester requesting client
    let requester = RequesterRequestingClient::new(
        server_url,
        rpc_provider,
        signer,
        SEPOLIA_UNIVERSAL_BOMBETTA_ADDRESS,
        SystemId::Risc0,
        validation_config,
        Risc0VerifierConstraints::for_network(network).into(),
    );

    // set intent builder defaults
    let builder_default = requester
        .builder
        .clone()
        .auction_length(auction_length) // 30 secs
        .reward_token_address(reward_token_address)
        .reward_token_decimals(reward_token_decimals);

    // intent builder that extends from default builder
    let builder = builder_default.clone();

    // system inputs
    let proof_info = serde_json::to_value(Risc0ProofParams { elf, inputs })?;

    // load verification commitments
    let public_inputs_commitment_preimage = DynSolValue::Tuple(vec![
        DynSolValue::FixedBytes(risc0_image_id, 32),
        DynSolValue::Uint(proof_input, 256),
    ]);
    let public_inputs_commitment_digest =
        Sha256::digest(public_inputs_commitment_preimage.abi_encode());
    let public_inputs_commitment = B256::from_slice(public_inputs_commitment_digest.as_slice());

    // build proof commitment's verifier details
    let verifier_details = VerifierDetails {
        verifier: verifier_address,
        selector: verify_function_selector,
        isShaCommitment: is_sha_commitment,
        inputsOffset: inputs_offset,
        inputsLength: inputs_length,
        hasPartialCommitmentResultCheck: has_partial_commitment_result_check,
        submittedPartialCommitmentResultOffset: submitted_partial_commitment_result_offset,
        submittedPartialCommitmentResultLength: submitted_partial_commitment_result_length,
        predeterminedPartialCommitment: pre_determined_partial_commitment,
    };
    // set extra_data = abi encoded verifier details
    let extra_data = Bytes::from(VerifierDetails::abi_encode(&verifier_details));

    // finish building compute request
    let compute_request = builder
        .set_new_nonce()
        .await?
        .set_token_params(minimum_stake, min_reward_amount, max_reward_amount)
        .proving_time(proving_time)
        .system(proof_info)
        .set_verification_commitment_params(public_inputs_commitment, extra_data)
        .set_auction_timestamps_from_auction_length()
        .await?
        .build()?; // convert ComputeRequestBuilder into ComputeRequest

    // sign built compute request
    let signed_request = requester.sign(compute_request.clone()).await?;

    // validate before submitting
    requester.validate_request(&signed_request)?;

    // submit and track ComputeRequest
    requester
        .submit_and_track(signed_request, u64::from(auction_length))
        .await?;

    Ok(())
}
