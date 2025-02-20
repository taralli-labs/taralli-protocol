use alloy::{
    network::Ethereum,
    primitives::Address,
    providers::{Provider, ProviderBuilder},
    transports::Transport,
};
use axum::{
    body::{Body, BodyDataStream},
    extract::{Query, State},
    http::{Request, StatusCode},
    response::{IntoResponse, Response},
    Json, Router,
};
use serde::Deserialize;
use serde_json::json;

use axum::response::sse::{Event, Sse};
use taralli_primitives::{
    systems::{SystemId, SYSTEMS},
    validation::{
        offer::OfferSpecificConfig, request::RequestSpecificConfig, CommonValidationConfig,
        ValidationMetaConfig,
    },
};
use tokio_stream::wrappers::BroadcastStream;

use bytes::Bytes;
use futures::stream::MapOk;
use serde_json::Value;
use taralli_server::{config::Config, state::BaseState, subscription_manager::SubscriptionManager};
use tower_http::trace::TraceLayer;

use futures_util::stream::{StreamExt, TryStreamExt};
use std::sync::Arc;
use std::time::Duration;
use tower::util::ServiceExt;

pub const MAX_BODY_SIZE: usize = 1024 * 1024; // 1 MB limit

pub fn submit_request_body(input: String) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri("/submit/request")
        .header("Content-Type", "application/json")
        .body(Body::from(input))
        .unwrap()
}

pub fn subscribe_request_body(system_ids: &[SystemId]) -> Request<Body> {
    // Convert system IDs to query string
    let query = system_ids
        .iter()
        .map(|id| id.as_str().to_string())
        .collect::<Vec<_>>()
        .join(",");

    let uri = format!("/subscribe?system_ids={}", query);

    println!("Subscribe URI: {}", uri); // Debug

    Request::builder()
        .method("GET")
        .uri(uri)
        .header("Accept", "text/event-stream")
        .body(Body::empty())
        .unwrap()
}

pub async fn submit_handler_json<T, P>(
    app_state: State<ValueState<T, P>>,
    Json(request): Json<Value>,
) -> impl IntoResponse
where
    T: Transport + Clone,
    P: Provider<T> + Clone,
{
    // Extract proving_system_id from the request
    let system_id = match request.get("proving_system_id").and_then(|v| v.as_str()) {
        Some(id) => match SystemId::try_from(id) {
            Ok(id) => id,
            Err(_) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({
                        "message": "Invalid proving_system_id"
                    })),
                )
            }
        },
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "message": "Missing proving_system_id field"
                })),
            )
        }
    };

    match app_state
        .subscription_manager()
        .broadcast(system_id, request)
        .await
    {
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

#[derive(Debug, Deserialize)]
pub struct TestSubscribeQuery {
    pub system_ids: String,
}

pub async fn subscribe_handler_json<T, P>(
    app_state: State<ValueState<T, P>>,
    Query(params): Query<TestSubscribeQuery>,
) -> Sse<impl futures::Stream<Item = Result<Event, axum::Error>>>
where
    T: Transport + Clone,
    P: Provider<T> + Clone,
{
    tracing::debug!("Incoming system_ids: {:?}", params.system_ids);
    println!("Incoming system_ids: {:?}", params); // Debug
    let ids = params.system_ids.split(',').collect::<Vec<&str>>();
    let mut invalid_ids = Vec::new();
    let mut valid_ids = Vec::new();
    for id_str in ids {
        match SystemId::try_from(id_str) {
            Ok(id) => valid_ids.push(id),
            Err(_) => invalid_ids.push(id_str),
        }
    }

    println!("Valid IDs: {:?}", valid_ids); // Debug
    let subscription_manager = app_state.subscription_manager();

    // Subscribe
    let receivers = subscription_manager.subscribe_to_ids(&valid_ids).await;

    println!("Subscribed to {} systems", receivers.len()); // Debug
                                                           // Convert each receiver into a stream of SSE events
    let streams = receivers.into_iter().map(|rx| {
        BroadcastStream::new(rx).map(|result| {
            result
                .map_err(axum::Error::new)
                .and_then(|data| Event::default().json_data(data).map_err(axum::Error::new))
        })
    });

    // Merge all streams into one
    let merged_stream = futures::stream::select_all(streams);

    Sse::new(merged_stream)
}

pub async fn submit(app: Router, input: String) -> Response<Body> {
    app.oneshot(submit_request_body(input)).await.unwrap()
}

pub async fn subscribe(
    app: Router,
    system_ids: &[SystemId],
) -> MapOk<BodyDataStream, impl FnMut(Bytes) -> String> {
    let subscribe_response = app
        .clone()
        .oneshot(subscribe_request_body(system_ids))
        .await
        .unwrap();
    println!("Subscribe response: {:?}", subscribe_response);
    // log::debug!("Subscribe response: {:?}", subscribe_response);
    assert_eq!(subscribe_response.status(), StatusCode::OK);
    let body_stream = subscribe_response.into_body().into_data_stream();
    // Map the stream of Bytes into SSE Event
    body_stream.map_ok(|bytes| String::from_utf8(bytes.to_vec()).unwrap())
}

#[derive(Clone)]
pub struct ValueState<T, P> {
    _base: BaseState<T, P>,
    subscription_manager: Arc<SubscriptionManager<Value>>,
}

impl<T, P> ValueState<T, P>
where
    T: Transport + Clone,
    P: Provider<T, Ethereum> + Clone,
{
    pub fn new(base: BaseState<T, P>, subscription_manager: SubscriptionManager<Value>) -> Self {
        Self {
            _base: base,
            subscription_manager: Arc::new(subscription_manager),
        }
    }

    pub fn subscription_manager(&self) -> Arc<SubscriptionManager<Value>> {
        self.subscription_manager.clone()
    }
}

pub async fn setup_app(size: Option<usize>) -> Router {
    let config = Config {
        server_port: 8080,
        rpc_url: "http://localhost:8545".to_owned(),
        log_level: "DEBUG".to_owned(),
        validation_timeout_seconds: 1,
        market_address: Address::default(),
        common_validation_config: CommonValidationConfig::default(),
        request_validation_config: RequestSpecificConfig::default(),
        offer_validation_config: OfferSpecificConfig::default(),
    };

    let rpc_provider = ProviderBuilder::new().on_http(config.rpc_url().unwrap());
    let subscription_manager: SubscriptionManager<Value> =
        SubscriptionManager::new(size.unwrap_or(1));

    subscription_manager.init_channels(&SYSTEMS).await;

    let validation_meta_config = ValidationMetaConfig {
        common: config.common_validation_config,
        request: config.request_validation_config,
        offer: config.offer_validation_config,
    };

    let base_state = BaseState::new(
        rpc_provider,
        config.market_address,
        Duration::from_secs(config.validation_timeout_seconds as u64),
        validation_meta_config,
    );

    let value_state = ValueState::new(base_state, subscription_manager);

    Router::new()
        .route("/submit/request", axum::routing::post(submit_handler_json))
        .route("/subscribe", axum::routing::get(subscribe_handler_json))
        .with_state(value_state)
        // .fallback(|| async { (StatusCode::NOT_FOUND, "Not Found") })
        .layer(TraceLayer::new_for_http())
}
