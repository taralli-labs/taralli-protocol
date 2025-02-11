use alloy::network::EthereumWallet;
use alloy::providers::ProviderBuilder;
use alloy::signers::local::PrivateKeySigner;
use color_eyre::Result;
use dotenv::dotenv;
use std::env;
use std::str::FromStr;
use taralli_primitives::markets::UNIVERSAL_BOMBETTA_ADDRESS;
use taralli_primitives::validation::request::RequestValidationConfig;
use taralli_provider::config::ProviderConfig;
use taralli_provider::workers::risc0::remote::Risc0RemoteProver;
use taralli_provider::workers::risc0::Risc0Worker;
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
    // build provider
    let rpc_provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .wallet(wallet)
        .on_http(rpc_url);
    // market contract
    let market_address = UNIVERSAL_BOMBETTA_ADDRESS;

    // build provider client config
    let config = ProviderConfig::new(
        rpc_provider,
        market_address,
        server_url,
        RequestValidationConfig::default(),
    );

    // setup prover
    let risc0_bonsai_prover = Risc0RemoteProver;

    // instantiate provider client
    let provider_client = ProviderClient::builder(config)
        .with_worker("risc0", Risc0Worker::new(risc0_bonsai_prover))?
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
