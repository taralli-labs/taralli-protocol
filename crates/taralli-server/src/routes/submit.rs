use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde_json::json;
use taralli_primitives::alloy::{providers::Provider, transports::Transport};
use taralli_primitives::compression_utils::intents::{
    ComputeOfferCompressed, ComputeRequestCompressed,
};

use crate::error::{Result, ServerError};
use crate::extracted_intents::{ExtractedOffer, ExtractedRequest};
use crate::state::offer::OfferState;
use crate::state::request::RequestState;
use crate::subscription_manager::BroadcastedMessage;
use crate::validation::{validate_partial_offer, validate_partial_request};

/// submit ComputeRequest
pub async fn submit_request_handler<T: Transport + Clone, P: Provider<T> + Clone>(
    State(state): State<RequestState<T, P>>,
    ExtractedRequest {
        partial_request,
        system_bytes,
    }: ExtractedRequest,
) -> Result<impl IntoResponse> {
    tracing::info!("ComputeRequest submitted: {:?}", partial_request);
    validate_partial_request(&partial_request, &state).await?;
    tracing::info!("compute request validated, broadcasting");

    let request_compressed =
        ComputeRequestCompressed::from((partial_request.clone(), system_bytes));

    let request_serialized = bincode::serialize(&request_compressed).map_err(|_e| {
        tracing::info!("Couldn't serialize partial request: {:?}", partial_request);
        ServerError::SerializationError(
            "Couldn't serialize request before broadcasting".to_string(),
        )
    })?;

    let message_to_broadcast = BroadcastedMessage {
        content: request_serialized,
        subscribed_to: partial_request.system_id.as_bit(),
    };

    match state.subscription_manager().broadcast(message_to_broadcast) {
        Ok(recv_count) => Ok((
            StatusCode::OK,
            Json(json!({
                "message": "compute request broadcast to providers",
                "broadcast_receivers": recv_count
            })),
        )),
        Err(_) => Err(ServerError::NoProvidersAvailable()),
    }
}

/// submit ComputeOffer
pub async fn submit_offer_handler<T: Transport + Clone, P: Provider<T> + Clone>(
    State(state): State<OfferState<T, P>>,
    ExtractedOffer {
        partial_offer,
        system_bytes,
    }: ExtractedOffer,
) -> Result<impl IntoResponse> {
    tracing::info!("ComputeOffer submitted: {:?}", partial_offer);
    validate_partial_offer(&partial_offer, &state).await?;
    tracing::info!("compute offer validated, storing");

    let offer_compressed = ComputeOfferCompressed::from((partial_offer, system_bytes));

    match state.intent_db().store_offer(&offer_compressed).await {
        Ok(_) => Ok((
            StatusCode::CREATED,
            Json(json!({"message": "Offer stored successfully"})),
        )),
        Err(e) => Err(e),
    }
}
