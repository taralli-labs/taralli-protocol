use alloy::{
    primitives::Address, providers::Provider, providers::ProviderBuilder, transports::Transport,
};
use axum::{
    body::{Body, BodyDataStream},
    extract::State,
    http::{Request, StatusCode},
    response::{IntoResponse, Response},
    Json, Router,
};
use serde::Serialize;
use serde_json::json;

use axum::response::sse::{Event, Sse};
use tokio_stream::wrappers::BroadcastStream;

use bytes::Bytes;
use futures::stream::MapOk;
use serde_json::Value;
use taralli_server::{
    app_state::AppState, config::Config, subscription_manager::SubscriptionManager,
};
use tower_http::trace::TraceLayer;

use futures_util::stream::{StreamExt, TryStreamExt};
use std::sync::Arc;
use std::time::Duration;
use tower::util::ServiceExt;

pub const MAX_BODY_SIZE: usize = 1024 * 1024; // 1 MB limit

pub fn submit_request_body(input: Option<String>) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri("/submit")
        .header("Content-Type", "application/json")
        .body(Body::from(input.unwrap_or("{}".to_owned())))
        .unwrap()
}

pub fn subscribe_request_body() -> Request<Body> {
    Request::builder()
        .method("GET")
        .uri("/subscribe")
        .header("Accept", "text/event-stream")
        .body(Body::empty())
        .unwrap()
}

pub async fn submit_handler_json<T, P, M>(
    app_state: State<AppState<T, P, M>>,
    Json(request): Json<M>,
) -> impl IntoResponse
where
    T: Transport + Clone,
    P: Provider<T> + Clone,
    M: Clone + Send + Sync + 'static + Serialize,
{
    match app_state.subscription_manager().broadcast(request) {
        Ok(recv_count) => (
            StatusCode::OK,
            Json(json!({
                "message": "Request accepted and submitted.",
                "broadcast_receivers": recv_count
            })),
        ),
        Err(e) => (
            StatusCode::ACCEPTED,
            Json(json!({
                "message": "Request accepted, but there were no receivers to submit to.",
                "broadcast_error": e.to_string()
            })),
        ),
    }
}

pub async fn subscribe_handler_json<T, P, M>(
    app_state: State<AppState<T, P, M>>,
) -> Sse<impl futures::Stream<Item = Result<Event, axum::Error>>>
where
    T: Transport + Clone,
    P: Provider<T> + Clone,
    M: Clone + Send + Sync + 'static + Serialize,
{
    let recv_new = app_state.subscription_manager().add_subscription();
    Sse::new(BroadcastStream::new(recv_new).map(|result| {
        result
            .map_err(axum::Error::new)
            .and_then(|data| Event::default().json_data(data).map_err(axum::Error::new))
    }))
}

pub async fn submit(app: Router, input: Option<String>) -> Response<Body> {
    app.oneshot(submit_request_body(Some(input.unwrap_or("{}".to_owned()))))
        .await
        .unwrap()
}

pub async fn subscribe(app: Router) -> MapOk<BodyDataStream, impl FnMut(Bytes) -> String> {
    let subscribe_response = app.clone().oneshot(subscribe_request_body()).await.unwrap();
    assert_eq!(subscribe_response.status(), StatusCode::OK);
    let body_stream = subscribe_response.into_body().into_data_stream();
    // Map the stream of Bytes into SSE Event
    body_stream.map_ok(|bytes| String::from_utf8(bytes.to_vec()).unwrap())
}

pub async fn setup_app(size: Option<usize>) -> Router {
    let config = Config {
        server_port: 8080,
        admin_port: None,
        rpc_url: "http://localhost:8545".to_owned(),
        log_level: "DEBUG".to_owned(),
        validation_timeout_seconds: 1,
        minimum_allowed_proving_time: 0,
        maximum_allowed_start_delay: 0,
        maximum_allowed_stake: 0,
        market_address: Address::default(),
        proving_system_ids: vec![],
    };
    let rpc_provider = ProviderBuilder::new().on_http(config.rpc_url().unwrap());
    let subscription_manager: SubscriptionManager<Value> =
        SubscriptionManager::new(size.unwrap_or(1));
    let app_state = AppState::new(
        rpc_provider,
        Arc::new(subscription_manager),
        config.market_address,
        config.proving_system_ids.clone(),
        0,
        0,
        0,
        Duration::from_secs(10),
    );

    Router::new()
        .route("/submit", axum::routing::post(submit_handler_json))
        .route("/subscribe", axum::routing::get(subscribe_handler_json))
        .with_state(app_state)
        .layer(TraceLayer::new_for_http())
}
