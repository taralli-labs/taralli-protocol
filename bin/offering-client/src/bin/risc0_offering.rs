use alloy::dyn_abi::DynSolValue;
use alloy::primitives::{address, fixed_bytes, Bytes, FixedBytes, B256, U256};
use alloy::providers::ProviderBuilder;
use alloy::signers::k256::sha2::Sha256;
use alloy::signers::local::PrivateKeySigner;
use alloy::sol_types::SolValue;
use color_eyre::Result;
use dotenv::dotenv;
use risc0_zkvm::ProverOpts;
use serde_json::Value;
use sha3::Digest;
use std::env;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;
use taralli_client::client::provider::offering::ProviderOfferingClient;
use taralli_client::intent_builder::IntentBuilder;
use taralli_primitives::abi::universal_porchetta::VerifierDetails;
use taralli_primitives::markets::SEPOLIA_UNIVERSAL_PORCHETTA_ADDRESS;
use taralli_primitives::systems::arkworks::ArkworksProofParams;
use taralli_primitives::systems::SystemId;
use taralli_primitives::validation::offer::OfferValidationConfig;
use taralli_primitives::validation::BaseValidationConfig;
use taralli_worker::risc0::local::Risc0LocalProver;
use taralli_worker::risc0::Risc0Worker;
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
    let priv_key = &env::var("PROVIDER_PRIVATE_KEY")?; // private key

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
    let inputs = serde_json::from_reader(inputs_reader)?;

    // on chain proof request data
    let test_token_address = address!("b54061f59AcF94f86ee414C9a220aFFE8BbE6B35");
    let test_token_decimals = 18u8;
    let reward_amount = U256::from(10); // 10 wei of tokens
    let stake_amount = U256::from(1); // 1 wei of tokens
    let proving_time = 60u32; // 1 min
    let auction_length = 90u32; // 3 min
    let verifier_address = address!("31766974fb795dF3f7d0c010a3D5c55e4bd8113e");
    let verify_function_selector: FixedBytes<4> = fixed_bytes!("ab750e75");
    let inputs_offset = U256::from(32);
    let inputs_length = U256::from(64);
    let is_sha_commitment = true;

    // signer
    let signer = PrivateKeySigner::from_str(priv_key)?;

    // build provider
    let rpc_provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .on_http(rpc_url);

    let validation_config = OfferValidationConfig {
        base: BaseValidationConfig::default(),
        minimum_allowed_stake: U256::from(1), // 1 wei of tokens
        maximum_allowed_reward: U256::from(100000000000000000000u128), // 100 tokens
    };

    // setup prover
    let risc0_prover = Risc0LocalProver::new(ProverOpts::groth16());

    let worker = Arc::new(Risc0Worker::new(risc0_prover));

    let provider = ProviderOfferingClient::new(
        server_url,
        rpc_provider,
        signer,
        SEPOLIA_UNIVERSAL_PORCHETTA_ADDRESS,
        SystemId::Risc0,
        worker,
        validation_config,
    );

    // set builder defaults
    let builder_default = provider
        .builder
        .clone()
        .auction_length(auction_length) // 30 secs
        .reward_token_address(test_token_address)
        .reward_token_decimals(test_token_decimals);

    // builder that extends from default builder
    let builder = builder_default.clone();

    // craft proving system information json here
    let proof_info = serde_json::to_value(ArkworksProofParams { r1cs, wasm, inputs })?;

    // load verification commitments
    // abi encode public input number
    // Extract the number directly from the JSON array
    let public_input_str = public_inputs[0]
        .as_str()
        .unwrap_or("failed to grab number from public.json");
    let u256_public_input = U256::from_str(public_input_str)?;
    let public_inputs_commitment_preimage =
        DynSolValue::Tuple(vec![DynSolValue::Uint(u256_public_input, 256)]);
    let public_inputs_commitment_digest =
        Sha256::digest(public_inputs_commitment_preimage.abi_encode());
    let public_inputs_commitment = B256::from_slice(public_inputs_commitment_digest.as_slice());

    // build verifier details using external tool
    let verifier_details = VerifierDetails {
        verifier: verifier_address,
        selector: verify_function_selector,
        isShaCommitment: is_sha_commitment,
        inputsOffset: inputs_offset,
        inputsLength: inputs_length,
    };
    // set extra_data = abi encoded verifier details
    let extra_data = Bytes::from(VerifierDetails::abi_encode(&verifier_details));

    // finish building proof request
    let compute_offer = builder
        .set_new_nonce()
        .await?
        .set_token_params(reward_amount, test_token_address, stake_amount)
        .proving_time(proving_time)
        .system(proof_info)
        .set_verification_commitment_params(public_inputs_commitment, extra_data)
        .set_auction_timestamps_from_auction_length()
        .await?
        .build()?; // convert ComputeOfferBuilder into ComputeOffer

    // sign built offer
    let signed_offer = provider.sign(compute_offer.clone()).await?;

    // validate before submitting
    provider.validate_offer(&signed_offer)?;

    println!(
        "signed offer proof commitment: {:?}",
        signed_offer.proof_offer
    );

    // TODO: Add a retry policy
    provider
        .submit_and_track(signed_offer, auction_length as u64)
        .await?;
    Ok(())
}
