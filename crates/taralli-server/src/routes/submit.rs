use crate::brotli::SubmittedRequest;
use crate::{app_state::AppState, error::ServerError, validation::validate_proof_request};
use alloy::{providers::*, transports::Transport};
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde_json::json;
use taralli_primitives::RequestCompressed;

pub async fn submit_handler<T: Transport + Clone, P: Provider<T> + Clone>(
    app_state: State<AppState<T, P>>,
    SubmittedRequest {
        partial_request,
        proving_system_information_bytes,
    }: SubmittedRequest,
) -> Result<impl IntoResponse, impl IntoResponse> {
    let timeout = app_state.validation_timeout_seconds();

    tracing::info!("Validating proof request");
    match validate_proof_request(&partial_request, &app_state, timeout).await {
        Ok(()) => {
            tracing::debug!("Validation successful, attempting to broadcast");
            let request_compressed =
                RequestCompressed::from((partial_request.clone(), proving_system_information_bytes));
            let request_serialized = bincode::serialize(&request_compressed).map_err(|_e| {
                tracing::info!("Couldn't serialize partial request: {:?}", partial_request);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "error": "Couldn't serialize request before broadcasting to Proof Providers"
                    })),
                )
            })?;
            match app_state
                .subscription_manager()
                .broadcast(request_serialized)
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
                            "broadcast_receivers": recv_count
                        })),
                    ))
                }
                Err(_) => {
                    tracing::debug!("No active subscribers to receive the broadcast");
                    Err((
                        StatusCode::BAD_REQUEST,
                        Json(json!({
                            "error": "No providers subscribed to listen for this request."
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
