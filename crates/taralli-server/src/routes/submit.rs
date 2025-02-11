use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde_json::json;
use taralli_primitives::alloy::{providers::Provider, transports::Transport};
use taralli_primitives::intents::{ComputeOffer, ComputeRequest};
use taralli_primitives::systems::ProvingSystemParams;

use crate::error::{Result, ServerError};
use crate::state::offer::OfferState;
use crate::state::request::RequestState;
use crate::validation::validate_intent;

pub async fn submit_request_handler<T: Transport + Clone, P: Provider<T> + Clone>(
    State(app_state): State<RequestState<T, P>>,
    Json(request): Json<ComputeRequest<ProvingSystemParams>>,
) -> Result<impl IntoResponse> {
    validate_intent(&request, &app_state).await?;
    match app_state
        .subscription_manager()
        .broadcast(request.proving_system_id, request)
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
    State(app_state): State<OfferState<T, P>>,
    Json(offer): Json<ComputeOffer<ProvingSystemParams>>,
) -> Result<impl IntoResponse> {
    validate_intent(&offer, &app_state).await?;

    match app_state.intent_db().store_offer(&offer).await {
        Ok(_) => Ok((
            StatusCode::CREATED,
            Json(json!({"message": "Offer stored successfully"})),
        )),
        Err(e) => Err(e),
    }
}
