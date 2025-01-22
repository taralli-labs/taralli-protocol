use alloy::dyn_abi::DynSolValue;
use alloy::primitives::{address, fixed_bytes, Bytes, FixedBytes, B256, U256};
use alloy::providers::ProviderBuilder;
use alloy::signers::k256::sha2::Sha256;
use alloy::signers::local::PrivateKeySigner;
use alloy::sol_types::SolValue;
use color_eyre::Result;
use dotenv::dotenv;
use serde_json::Value;
use sha3::Digest;
use std::env;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::str::FromStr;
use taralli_primitives::abi::universal_bombetta::VerifierDetails;
use taralli_primitives::market::UNIVERSAL_BOMBETTA_ADDRESS;
use taralli_primitives::systems::arkworks::ArkworksProofParams;
use taralli_primitives::systems::ProvingSystemId;
use taralli_requester::config::RequesterConfig;
use taralli_requester::RequesterClient;
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
    let rpc_url = Url::parse(&env::var("RPC_URL")?)?; // Holesky testnet
    let priv_key = &env::var("REQUESTER_PRIVATE_KEY")?; // Holesky private key

    // proving system information data
    let r1cs_data_path = Path::new("./contracts/test-proof-data/groth16/multiplier2.r1cs");
    let proof_inputs_file =
        File::open("./contracts/test-proof-data/groth16/multiplier2_js/input.json")?;
    let proof_public_inputs_file = File::open("./contracts/test-proof-data/groth16/public.json")?;
    let wasm_path =
        Path::new("./contracts/test-proof-data/groth16/multiplier2_js/multiplier2.wasm");
    // buf readers
    let public_inputs_reader = BufReader::new(proof_public_inputs_file);
    let inputs_reader = BufReader::new(proof_inputs_file);

    // decode proof input data
    let r1cs = std::fs::read(r1cs_data_path)?;
    let public_inputs: Value = serde_json::from_reader(public_inputs_reader)?;
    let wasm: Vec<u8> = std::fs::read(wasm_path)?;
    let input = serde_json::from_reader(inputs_reader)?;

    // on chain proof request data
    let market_address = UNIVERSAL_BOMBETTA_ADDRESS;
    let reward_token_address = address!("89fF1B147026815cf497AA45D4FDc2DF51Ed7f00");
    let reward_token_decimals = 18u8;
    let max_reward_amount = U256::from(100e18); // 100 tokens
    let min_reward_amount = U256::from(10); // 10 wei of tokens
    let minimum_stake = 1; // 1 wei, for testing
    let proving_time = 60u32; // 1 min
    let auction_length = 60u32; // 1 min
    let verifier_address = address!("3D48eB902f38fCF16C2fD9F42cb088d301D16c94");
    let verify_function_selector: FixedBytes<4> = fixed_bytes!("43753b4d");
    let public_inputs_offset = U256::from(256);
    let public_inputs_length = U256::from(32);
    let is_sha_commitment = false;
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
        ProvingSystemId::Arkworks,
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

    // craft proving system information json here
    let proof_info = serde_json::to_value(ArkworksProofParams { r1cs, wasm, input })?;

    // load verification commitments
    // abi encode public input number
    // Extract the number directly from the JSON array
    let public_input_str = public_inputs[0]
        .as_str()
        .unwrap_or("failed to grab number from public.json");
    let u256_public_input = U256::from_str(public_input_str)?;
    let public_inputs_commitment_preimage =
        DynSolValue::Tuple(vec![DynSolValue::Uint(u256_public_input, 256)]);
    // sha256(abi.encode(imageId, proofInputHash))
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
