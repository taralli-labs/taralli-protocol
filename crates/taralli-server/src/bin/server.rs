use alloy::providers::ProviderBuilder;
use axum::{
    http::{Response, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use color_eyre::{eyre::Context, Result};
use serde_json::json;
use std::{sync::Arc, time::Duration};
use taralli_server::{
    app_state::{AppState, AppStateConfig},
    config::Config,
    routes::{submit::submit_handler, subscribe::websocket_subscribe_handler},
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

    let rpc_provider = ProviderBuilder::new().on_http(config.rpc_url()?);
    let subscription_manager: Arc<SubscriptionManager> = Default::default();

    // initialize state
    let app_state = AppState::new(AppStateConfig {
        rpc_provider,
        subscription_manager,
        market_address: config.market_address,
        proving_system_ids: config.proving_system_ids,
        minimum_allowed_proving_time: config.minimum_allowed_proving_time,
        maximum_allowed_start_delay: config.maximum_allowed_start_delay,
        maximum_allowed_stake: config.maximum_allowed_stake,
        validation_timeout_seconds: Duration::from_secs(config.validation_timeout_seconds as u64),
    });

    let app = Router::new()
        .route("/submit", post(submit_handler))
        .route("/subscribe", get(websocket_subscribe_handler))
        .with_state(app_state)
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
