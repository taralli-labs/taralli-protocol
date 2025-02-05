use alloy::providers::ProviderBuilder;
use axum::{
    http::{Response, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use color_eyre::{eyre::Context, Result};
use serde_json::json;
use std::time::Duration;
use taralli_primitives::{
    intents::ComputeRequest, systems::ProvingSystemParams, validation::ValidationMetaConfig,
};
use taralli_server::{
    config::Config,
    postgres::Db,
    routes::submit::{submit_offer_handler, submit_request_handler},
    state::{offer::OfferState, request::RequestState, BaseState},
    subscription_manager::SubscriptionManager,
};
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing::info;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    // Load configuration
    let config = Config::from_file("config.json").context("Failed to load config")?;

    // tracing
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_max_level(config.log_level()?)
        .init();

    // Setup validation config
    let validation_config = ValidationMetaConfig {
        common: config.common_validation_config.clone(),
        request: config.request_validation_config.clone(),
        offer: config.offer_validation_config.clone(),
    };

    let rpc_provider = ProviderBuilder::new().on_http(config.rpc_url()?);

    // setup intent subscription manager
    let subscription_manager: SubscriptionManager<ComputeRequest<ProvingSystemParams>> =
        Default::default();

    // initialize intent database
    let offer_db = Db::new().await;

    let base_state = BaseState::new(
        rpc_provider.clone(),
        config.market_address,
        Duration::from_secs(config.validation_timeout_seconds as u64),
        validation_config.clone(),
    );

    let request_state = RequestState::new(base_state.clone(), subscription_manager);

    let offer_state = OfferState::new(base_state, offer_db);

    // Create separate routers for each intent type
    let request_routes = Router::new()
        .route("/submit/request", post(submit_request_handler))
        .with_state(request_state);

    let offer_routes = Router::new()
        .route("/submit/offer", post(submit_offer_handler))
        .with_state(offer_state);

    // Merge routers
    let app = request_routes
        .merge(offer_routes)
        .layer(TraceLayer::new_for_http())
        .fallback(get(fallback));

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
