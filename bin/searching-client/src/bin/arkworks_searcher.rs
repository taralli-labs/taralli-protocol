use alloy::network::EthereumWallet;
use alloy::providers::ProviderBuilder;
use alloy::signers::local::PrivateKeySigner;
use color_eyre::Result;
use dotenv::dotenv;
use std::env;
use std::str::FromStr;
use taralli_client::client::requester::searching::RequesterSearchingClient;
use taralli_primitives::alloy::primitives::U256;
use taralli_primitives::markets::SEPOLIA_UNIVERSAL_PORCHETTA_ADDRESS;
use taralli_primitives::systems::SystemId;
use taralli_primitives::validation::offer::{
    ComputeOfferValidator, OfferValidationConfig, OfferVerifierConstraints,
};
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

    // build signer
    let signer = PrivateKeySigner::from_str(priv_key)?;
    // build wallet for sending txs
    let wallet = EthereumWallet::new(signer.clone());
    // build rpc provider
    let rpc_provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .wallet(wallet)
        .on_http(rpc_url);

    // validation config to check selected offers are correct
    let validation_config = OfferValidationConfig {
        base: BaseValidationConfig::default(),
        minimum_allowed_stake: U256::from(1), // 1 wei of tokens
        maximum_allowed_reward: U256::from(100000000000000000000u128), // 100 tokens
    };

    // verifier constraints
    let verifier_constraints = OfferVerifierConstraints::default();

    // validator
    let _validator = ComputeOfferValidator::new(validation_config.clone(), verifier_constraints);

    // instantiate requester searching client
    let searcher_client: RequesterSearchingClient<_, _, _, _> = RequesterSearchingClient::new(
        server_url,
        rpc_provider,
        signer.clone(),
        SEPOLIA_UNIVERSAL_PORCHETTA_ADDRESS,
        SystemId::Arkworks,
        validation_config,
    );

    // run searcher client
    // Query the server at the selected system id, filter through various criteria based what offers are returned from the query.
    // Then analyze the selected offer, if viable then bid upon it. Once a bid is submitted track the status of the resolution of
    // the compute offer.
    searcher_client.run().await?;

    Ok(())
}
