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
use taralli_primitives::markets::{Network, SEPOLIA_UNIVERSAL_PORCHETTA_ADDRESS};
use taralli_primitives::systems::sp1::{
    Sp1Config, Sp1Mode, Sp1ProofParams, Sp1VerifierConstraints,
};
use taralli_primitives::systems::SystemId;
use taralli_primitives::validation::offer::OfferValidationConfig;
use taralli_primitives::validation::BaseValidationConfig;
use taralli_worker::sp1::local::Sp1LocalProver;
use taralli_worker::sp1::Sp1Worker;
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
    let sp1_program_path = Path::new("./contracts/test-proof-data/sp1/fibonacci-program");
    // proof input(s)
    let inputs = 1000u32;
    // load elf binary
    let elf = std::fs::read(sp1_program_path)?;

    // proof commitment data
    let reward_token_address = address!("b54061f59AcF94f86ee414C9a220aFFE8BbE6B35");
    let reward_token_decimals = 18u8;
    let reward_amount = U256::from(10); // 10 wei of tokens
    let stake_token_address = address!("b54061f59AcF94f86ee414C9a220aFFE8BbE6B35");
    let stake_token_decimals = 18u8;
    let stake_amount = U256::from(1); // 1 wei of tokens
    let proving_time = 60u32; // 1 min
    let auction_length = 90u32; // 3 min

    // SP1 sepolia groth16 verifier
    let verifier_address = address!("E780809121774D06aD9B0EEeC620fF4B3913Ced1");
    // verifyProof(bytes32 programVKey,bytes calldata publicValues,bytes calldata proofBytes)
    let verify_function_selector: FixedBytes<4> = fixed_bytes!("41493c60");
    // offset and length to extract inputs field
    let inputs_offset = U256::from(0);
    let inputs_length = U256::from(64);
    // uses sha
    let is_sha_commitment = true;

    // network
    let network = Network::Sepolia;

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

    // setup sp1 prover
    let sp1_prover = Sp1LocalProver::new(false, sp1_sdk::SP1ProofMode::Groth16);
    // setup sp1 compute worker
    let worker = Arc::new(Sp1Worker::new(sp1_prover));

    // instantiate provider offering client
    let provider = ProviderOfferingClient::new(
        server_url,
        rpc_provider,
        signer,
        SEPOLIA_UNIVERSAL_PORCHETTA_ADDRESS,
        SystemId::Sp1,
        worker,
        validation_config,
        Sp1VerifierConstraints::for_network(network).into(),
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
    let proof_info = serde_json::to_value(Sp1ProofParams {
        elf,
        inputs: inputs.to_le_bytes().to_vec(),
        config: Sp1Config {
            mode: Sp1Mode::Groth16,
        },
    })?;

    // load verification commitments
    let public_inputs_commitment_preimage =
        DynSolValue::Tuple(vec![DynSolValue::Bytes(inputs.to_le_bytes().to_vec())]);
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
    provider.validate_offer(&signed_offer)?;

    println!(
        "signed offer proof commitment: {:?}",
        signed_offer.proof_offer
    );

    // submit and track ComputeOffer
    provider
        .submit_and_track(signed_offer, u64::from(auction_length))
        .await?;
    Ok(())
}
