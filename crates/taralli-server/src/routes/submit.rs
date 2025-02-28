use crate::extracted_request::ExtractedRequest;
use crate::subscription_manager::BroadcastedMessage;
use crate::{app_state::AppState, error::ServerError, validation::validate_proof_request};
use alloy::{providers::*, transports::Transport};
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde_json::json;
use taralli_primitives::RequestCompressed;

pub async fn submit_handler<T: Transport + Clone, P: Provider<T> + Clone>(
    app_state: State<AppState<T, P>>,
    ExtractedRequest {
        partial_request,
        proving_system_information_bytes,
    }: ExtractedRequest,
) -> Result<impl IntoResponse, impl IntoResponse> {
    let timeout = app_state.validation_timeout_seconds();

    tracing::info!("Validating proof request: {:?}", partial_request);
    match validate_proof_request(&partial_request, &app_state, timeout).await {
        Ok(()) => {
            tracing::debug!("Validation successful, attempting to broadcast");
            let request_compressed = RequestCompressed::from((
                partial_request.clone(),
                proving_system_information_bytes,
            ));
            let request_serialized = bincode::serialize(&request_compressed).map_err(|_e| {
                tracing::info!("Couldn't serialize partial request: {:?}", partial_request);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "error": "Couldn't serialize request before broadcasting to Proof Providers"
                    })),
                )
            })?;
            let message_to_broadcast = BroadcastedMessage {
                content: request_serialized,
                subscribed_to: partial_request.proving_system_id.as_bit(),
            };
            match app_state
                .subscription_manager()
                .broadcast(message_to_broadcast)
            {
                Ok(recv_count) => {
                    tracing::info!(
                        "Submitted request was broadcast to {} receivers",
                        recv_count
                    );
                    Ok((
                        StatusCode::OK,
                        Json(json!({
                            "message": "Proof request accepted and submitted to Proof Providers.",
                            "broadcasted_to": recv_count
                        })),
                    ))
                }
                Err(_) => {
                    tracing::debug!("No active subscribers to receive the broadcast");
                    Err((
                        StatusCode::OK,
                        Json(json!({
                            "message": "No providers subscribed to listen for this request."
                        })),
                    ))
                }
            }
        }
        Err(e) => {
            tracing::warn!("Validation failed: {:?}", e);
            let status = match e {
                ServerError::ValidationTimeout(_) => StatusCode::REQUEST_TIMEOUT,
                _ => StatusCode::BAD_REQUEST,
            };

            Err((
                status,
                Json(json!({
                    "error": e.to_string(),
                    "error_type": format!("{:?}", e)
                })),
            ))
        }
    }
}
