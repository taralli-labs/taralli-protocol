use alloy::network::EthereumWallet;
use alloy::primitives::address;
use alloy::providers::ProviderBuilder;
use alloy::signers::local::PrivateKeySigner;
use color_eyre::Result;
use dotenv::dotenv;
use ethers::signers::LocalWallet;
use risc0_zkvm::ProverOpts;
use sp1_sdk::ProverClient;
use std::env;
use std::str::FromStr;
use taralli_provider::config::ProviderConfig;
use taralli_provider::workers::aligned_layer::AlignedLayerWorker;
use taralli_provider::workers::arkworks::ArkworksWorker;
use taralli_provider::workers::risc0::Risc0Worker;
use taralli_provider::workers::sp1::Sp1Worker;
use taralli_provider::ProviderClient;
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
    let priv_key = &env::var("PROVIDER_PRIVATE_KEY")?; // Holesky provider private key

    // build signer
    let signer = PrivateKeySigner::from_str(priv_key)?;
    // build wallet for sending txs
    let wallet = EthereumWallet::new(signer.clone());
    // need dummy ethers wallet for aligned layer sdk
    let ethers_wallet = priv_key.parse::<LocalWallet>()?;
    // build provider
    let rpc_provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .wallet(wallet)
        .on_http(rpc_url.clone());

    // market contract
    let market_address = address!("e05e737478E4f0b886981aD85CF9a59D55413e8b");

    // build provider client config
    let config = ProviderConfig::new(rpc_provider, market_address, server_url);

    // instantiate provider client
    let provider_client = ProviderClient::builder(config)
        .with_worker("arkworks", ArkworksWorker::new())?
        .with_worker("sp1", Sp1Worker::new(ProverClient::local()))?
        .with_worker("risc0", Risc0Worker::new(ProverOpts::groth16()))?
        .with_worker(
            "aligned-layer",
            AlignedLayerWorker::new(signer.address(), rpc_url.to_string(), ethers_wallet),
        )?
        .build();

    //// run provider client
    // Subscribes to the server and receives back an SSE stream or fails.
    // The client awaits the SSE stream returned by the server to receive newly
    // submitted requests. Upon receiving a new request its processed, which can
    // yield various types of errors/fail states. In the happy path, the request
    // is processed then successfully resolved onchain for a reward.
    provider_client.run().await?;

    Ok(())
}
