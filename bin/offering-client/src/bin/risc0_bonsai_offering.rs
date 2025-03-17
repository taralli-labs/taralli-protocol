use alloy::dyn_abi::DynSolValue;
use alloy::network::EthereumWallet;
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
use std::sync::Arc;
use taralli_client::client::provider::offering::ProviderOfferingClient;
use taralli_client::intent_builder::IntentBuilder;
use taralli_primitives::abi::universal_porchetta::VerifierDetails;
use taralli_primitives::markets::SEPOLIA_UNIVERSAL_PORCHETTA_ADDRESS;
use taralli_primitives::systems::risc0::Risc0ProofParams;
use taralli_primitives::systems::SystemId;
use taralli_primitives::validation::offer::OfferValidationConfig;
use taralli_primitives::validation::BaseValidationConfig;
use taralli_worker::risc0::remote::Risc0RemoteProver;
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
    let reward_amount = U256::from(10); // 10 wei of tokens
    let stake_token_address = address!("b54061f59AcF94f86ee414C9a220aFFE8BbE6B35");
    let stake_token_decimals = 18u8;
    let stake_amount = U256::from(1); // 1 wei of tokens
    let proving_time = 60u32; // 1 min
    let auction_length = 90u32; // 3 min
                                // Risc0 sepolia groth16 verifier
    let verifier_address = address!("AC292cF957Dd5BA174cdA13b05C16aFC71700327");
    // verify(bytes calldata seal, bytes32 imageId, bytes32 journalDigest)
    let verify_function_selector: FixedBytes<4> = fixed_bytes!("ab750e75");
    // offset and length to extract inputs field
    let inputs_offset = U256::from(32);
    let inputs_length = U256::from(64);
    // uses sha
    let is_sha_commitment = true;

    // signer
    let signer = PrivateKeySigner::from_str(priv_key)?;

    // build wallet for sending txs
    let wallet = EthereumWallet::new(signer.clone());

    // build rpc provider
    let rpc_provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .wallet(wallet)
        .on_http(rpc_url);

    // validation config to check offers are correct
    let validation_config = OfferValidationConfig {
        base: BaseValidationConfig::default(),
        minimum_allowed_stake: U256::from(1), // 1 wei of tokens
        maximum_allowed_reward: U256::from(100000000000000000000u128), // 100 tokens
    };

    // setup risc0 prover
    let risc0_prover = Risc0RemoteProver;
    // setup risc0 compute worker
    let worker = Arc::new(Risc0Worker::new(risc0_prover));

    // instantiate provider offering client
    let provider = ProviderOfferingClient::new(
        server_url,
        rpc_provider,
        signer,
        SEPOLIA_UNIVERSAL_PORCHETTA_ADDRESS,
        SystemId::Risc0,
        worker,
        validation_config,
    );

    // set intent builder defaults
    let builder_default = provider
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
    };
    // set extra_data = abi encoded verifier details
    let extra_data = Bytes::from(VerifierDetails::abi_encode(&verifier_details));

    // finish building compute offer
    let compute_offer = builder
        .set_new_nonce()
        .await?
        .set_token_params(
            reward_amount,
            stake_token_address,
            stake_token_decimals,
            stake_amount,
        )
        .proving_time(proving_time)
        .system(proof_info)
        .set_verification_commitment_params(public_inputs_commitment, extra_data)
        .set_auction_timestamps_from_auction_length()
        .await?
        .build()?; // convert ComputeOfferBuilder into ComputeOffer

    // sign built compute offer
    let signed_offer = provider.sign(compute_offer.clone()).await?;

    // validate before submitting
    provider.validate_offer(&signed_offer, &Default::default())?;

    tracing::info!(
        "signed offer proof commitment: {:?}",
        signed_offer.proof_offer
    );

    // submit and track ComputeOffer
    provider
        .submit_and_track(signed_offer, auction_length as u64)
        .await?;
    Ok(())
}
