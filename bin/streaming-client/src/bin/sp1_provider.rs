use alloy::network::EthereumWallet;
use alloy::providers::ProviderBuilder;
use alloy::signers::local::PrivateKeySigner;
use color_eyre::Result;
use dotenv::dotenv;
use std::collections::HashMap;
use std::env;
use std::str::FromStr;
use taralli_client::client::provider::streaming::ProviderStreamingClient;
use taralli_primitives::markets::SEPOLIA_UNIVERSAL_BOMBETTA_ADDRESS;
use taralli_primitives::systems::SystemId;
use taralli_primitives::validation::request::{
    RequestValidationConfig, RequestVerifierConstraints,
};
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
    let priv_key = &env::var("PROVIDER_PRIVATE_KEY")?; // provider private key

    // build signer
    let signer = PrivateKeySigner::from_str(priv_key)?;
    // build wallet for sending txs
    let wallet = EthereumWallet::new(signer.clone());
    // build rpc provider
    let rpc_provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .wallet(wallet)
        .on_http(rpc_url);

    // validation config to check incoming requests from streams are correct
    let validation_config = RequestValidationConfig {
        base: BaseValidationConfig::default(),
        maximum_allowed_stake: 10000000000000000000, // 10 ether
    };

    // setup sp1 prover
    let sp1_prover = Sp1LocalProver::new(false, sp1_sdk::SP1ProofMode::Groth16);

    // verifier constraints
    let mut verifier_constraints = HashMap::new();
    verifier_constraints.insert(SystemId::Sp1, RequestVerifierConstraints::default());

    // instantiate provider streaming client
    let provider_client = ProviderStreamingClient::new(
        server_url,
        rpc_provider,
        signer.clone(),
        SEPOLIA_UNIVERSAL_BOMBETTA_ADDRESS,
        validation_config,
        Some(verifier_constraints),
    )
    .with_worker(SystemId::Sp1, Sp1Worker::new(sp1_prover))?;

    // run provider client
    // Subscribes to the server and receives back a ws stream or fails.
    // The client awaits the ws stream returned by the server to receive newly
    // submitted compute requests. Upon receiving a new compute request its processed,
    // which can yield various types of errors/fail states. In the happy path, the
    // compute request is processed then successfully resolved onchain for a reward
    // within the market contract.
    provider_client.run().await?;

    Ok(())
}
