use alloy::network::EthereumWallet;
use alloy::providers::ProviderBuilder;
use alloy::signers::local::PrivateKeySigner;
use color_eyre::Result;
use dotenv::dotenv;
use risc0_zkvm::ProverOpts;
use std::env;
use std::str::FromStr;
use taralli_client::client::provider::streaming::ProviderStreamingClient;
use taralli_primitives::markets::{Network, SEPOLIA_UNIVERSAL_BOMBETTA_ADDRESS};
use taralli_primitives::systems::risc0::Risc0VerifierConstraints;
use taralli_primitives::systems::SystemId;
use taralli_primitives::validation::request::{ComputeRequestValidator, RequestValidationConfig};
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
    let priv_key = &env::var("PROVIDER_PRIVATE_KEY")?; // provider private key

    // network
    let network = Network::Sepolia;

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

    // setup risc0 prover
    let risc0_prover = Risc0LocalProver::new(ProverOpts::groth16());

    let validator = ComputeRequestValidator::new(
        validation_config.clone(),
        Risc0VerifierConstraints::for_network(network).into(),
    );

    // instantiate provider streaming client
    let provider_client = ProviderStreamingClient::new(
        server_url,
        rpc_provider,
        signer.clone(),
        SEPOLIA_UNIVERSAL_BOMBETTA_ADDRESS,
        validation_config,
    )
    .with_system_configuration(SystemId::Risc0, Risc0Worker::new(risc0_prover), validator)?;

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
