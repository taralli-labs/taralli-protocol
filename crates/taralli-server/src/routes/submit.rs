use crate::brotli::BrotliFile;
use crate::{app_state::AppState, error::ServerError, validation::validate_proof_request};
use alloy::{providers::*, transports::Transport};
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde_json::json;
use taralli_primitives::systems::ProvingSystemParams;
use taralli_primitives::Request;

pub async fn submit_handler<T: Transport + Clone, P: Provider<T> + Clone>(
    app_state: State<AppState<T, P>>,
    BrotliFile {
        compressed,
        decompressed,
    }: BrotliFile,
) -> Result<impl IntoResponse, impl IntoResponse> {
    let request: Request<ProvingSystemParams> =
        serde_json::from_slice(&decompressed).map_err(|e| {
            tracing::warn!("Failed to parse JSON: {:?}", e);
            (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "Invalid JSON after Brotli decompression" })),
            )
        })?;

    let timeout = app_state.validation_timeout_seconds();

    tracing::info!("Validating proof request");
    match validate_proof_request(&request, &app_state, timeout).await {
        Ok(()) => {
            tracing::debug!("Validation successful, attempting to broadcast");
            match app_state.subscription_manager().broadcast(compressed) {
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
