use axum::{http::StatusCode, response::IntoResponse, Json};
use serde_json::Value;
use taralli_primitives::PrimitivesError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ServerError {
    #[error("App state error: -> {0}")]
    AppStateError(String),
    #[error("Validation: Error when fetching latest block")]
    FetchLatestBlockTimestampError,
    #[error("Submit: validation timed out after {0} seconds")]
    ValidationTimeout(u64),
    #[error("Submit: validation error -> {0}")]
    ValidationError(String),
    #[error("Subscribe: invalid system id -> {0}")]
    SystemIdError(String),
    #[error("Subscription manager: no proof providers available for selected proving system.")]
    NoProvidersAvailable(),
    #[error("Broadcast failed: {0}")]
    BroadcastError(String),
    #[error("Query failed: {0}")]
    QueryError(String),
    #[error("Database error: {0}")]
    DatabaseError(String),
    #[error("Serialization error: {0}")]
    SerializationError(String),
    #[error("Primitives error: {0}")]
    PrimitivesError(#[from] PrimitivesError),
}

pub type Result<T> = core::result::Result<T, ServerError>;

impl IntoResponse for ServerError {
    fn into_response(self) -> axum::response::Response {
        let (status, error_message) = match &self {
            ServerError::ValidationTimeout(secs) => (
                StatusCode::REQUEST_TIMEOUT,
                format!("Validation timed out after {} seconds", secs),
            ),
            ServerError::NoProvidersAvailable() => (
                StatusCode::SERVICE_UNAVAILABLE,
                "No proof providers available".to_string(),
            ),
            ServerError::ValidationError(s) => (StatusCode::BAD_REQUEST, s.to_owned()),
            ServerError::BroadcastError(s) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Broadcast failed: {}", s),
            ),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error".to_string(),
            ),
        };
        (status, ApiResponse::failure(&error_message)).into_response()
    }
}

pub struct ApiResponse;
impl ApiResponse {
    pub fn failure(s: &str) -> Json<Value> {
        Json(serde_json::json!({"error": s}))
    }
}
