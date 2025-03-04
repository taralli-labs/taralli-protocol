use crate::brotli::BrotliFile;
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde_json::json;
use taralli_primitives::alloy::{providers::Provider, transports::Transport};
use taralli_primitives::intents::{offer::ComputeOffer, request::ComputeRequest};
use taralli_primitives::systems::SystemParams;

use crate::error::{Result, ServerError};
use crate::state::offer::OfferState;
use crate::state::request::RequestState;
use crate::validation::validate_intent;

pub async fn submit_request_handler<T: Transport + Clone, P: Provider<T> + Clone>(
    State(state): State<RequestState<T, P>>,
    BrotliFile {
        compressed,
        decompressed,
    }: BrotliFile,
) -> Result<impl IntoResponse> {
    tracing::info!("compute request submitted");
    let request: ComputeRequest<SystemParams> = serde_json::from_slice(&decompressed)
        .map_err(|e| {
            tracing::warn!("Failed to parse JSON: {:?}", e);
            (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "Invalid JSON after Brotli decompression" })),
            )
        })
        .map_err(|e| ServerError::DeserializationError(format!("error: {:?}", e)))?;

    validate_intent(&request, &state).await?;

    tracing::info!("compute request validated, broadcasting");

    match state
        .subscription_manager()
        .broadcast(request.system_id, compressed)
        .await
    {
        Ok(recv_count) => Ok((
            StatusCode::OK,
            Json(json!({
                "message": "Proof request broadcast to providers",
                "broadcast_receivers": recv_count
            })),
        )),
        Err(_) => Err(ServerError::NoProvidersAvailable()),
    }
}

pub async fn submit_offer_handler<T: Transport + Clone, P: Provider<T> + Clone>(
    State(state): State<OfferState<T, P>>,
    BrotliFile {
        compressed: _,
        decompressed,
    }: BrotliFile,
) -> Result<impl IntoResponse> {
    tracing::info!("compute offer submitted");
    let offer: ComputeOffer<SystemParams> = serde_json::from_slice(&decompressed)
        .map_err(|e| {
            tracing::warn!("Failed to parse JSON: {:?}", e);
            (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "Invalid JSON after Brotli decompression" })),
            )
        })
        .map_err(|e| ServerError::DeserializationError(format!("error: {:?}", e)))?;

    validate_intent(&offer, &state).await?;

    tracing::info!("compute offer validated, storing");

    match state.intent_db().store_offer(&offer).await {
        Ok(_) => Ok((
            StatusCode::CREATED,
            Json(json!({"message": "Offer stored successfully"})),
        )),
        Err(e) => Err(e),
    }
}
