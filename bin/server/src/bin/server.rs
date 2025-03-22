use alloy::providers::ProviderBuilder;
use axum::{
    http::{Response, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use color_eyre::{eyre::Context, Result};
use dotenv::dotenv;
use serde_json::json;
use std::{str::FromStr, time::Duration};
use taralli_primitives::env::Environment;
use taralli_server::{
    config::Config,
    postgres::Db,
    routes::{
        query::get_active_intents_by_id_handler,
        submit::{submit_offer_handler, submit_request_handler},
        subscribe::websocket_subscribe_handler,
    },
    state::{offer::OfferState, request::RequestState, BaseState},
    subscription_manager::SubscriptionManager,
};
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing::info;
use tracing_subscriber::EnvFilter;
use url::Url;

/// Taralli protocol server
/// Handles:
/// - submission of compute intents
/// - subscriptions thorugh websocket streams of compute intents across a given set of system IDs.
/// - storage of compute intents
#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    dotenv().ok();

    // Load configuration json
    let config = Config::from_file("config.json").context("Failed to load config")?;
    // Load rpc url used by the server's rpc provider (sepolia currently)

    let url_opt = std::env::var("RPC_URL");
    let rpc_url_string = match Environment::from_env_var() {
        Environment::Production => url_opt.expect("rpc url from env failed"),
        Environment::Development => url_opt.unwrap_or("http://rpc_url.com".to_string()),
    };
    let rpc_url = Url::from_str(&rpc_url_string).context("Invalid RPC URL")?;

    // setup tracing
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_max_level(config.log_level()?)
        .init();

    // Get the validation configs from the server config
    let validation_configs = config.get_validation_configs();

    tracing::info!("Setting up RPC provider");
    let rpc_provider = ProviderBuilder::new().on_http(rpc_url);

    // setup subscription manager
    tracing::info!("Setting up subscription manager");
    let subscription_manager: SubscriptionManager = Default::default();

    // initialize intent database
    tracing::info!("Setting up database");
    let intent_db = Db::new().await;

    tracing::info!("Setting up state");
    let base_state = BaseState::new(
        rpc_provider.clone(),
        config.markets,
        Duration::from_secs(config.validation_timeout_seconds as u64),
        validation_configs,
    );
    let request_state = RequestState::new(base_state.clone(), subscription_manager);
    let offer_state = OfferState::new(base_state, intent_db);

    tracing::info!("Setting up routers");
    // Create separate routers for each intent type
    let request_routes = Router::new()
        .route("/submit/request", post(submit_request_handler))
        .route("/subscribe", get(websocket_subscribe_handler))
        .with_state(request_state);
    let offer_routes = Router::new()
        .route("/submit/offer", post(submit_offer_handler))
        .route("/query/:system_id", get(get_active_intents_by_id_handler))
        .with_state(offer_state);

    tracing::info!("Merging routers");
    // Merge routers
    let app = request_routes
        .merge(offer_routes)
        .layer(TraceLayer::new_for_http())
        .fallback(get(fallback));

    tracing::info!("Starting server");
    let server_url = format!("0.0.0.0:{}", config.server_port);
    let listener = TcpListener::bind(server_url).await.context(format!(
        "Failed to bind server to port {}",
        config.server_port
    ))?;

    info!("Server running on port {}", config.server_port);
    axum::serve(listener, app).await?;

    Ok(())
}

async fn fallback() -> impl IntoResponse {
    Response::builder()
        .header("Content-Type", "application/json")
        .status(StatusCode::NOT_FOUND)
        .body(json!("404 Not Found").to_string())
        .expect("response building should not fail")
}
