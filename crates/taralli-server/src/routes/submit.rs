use alloy::{providers::*, transports::Transport};
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde_json::json;
use taralli_primitives::taralli_systems::traits::ProvingSystemInformation;
use taralli_primitives::ProofRequest;

use crate::{app_state::AppState, error::ServerError, validation::validate_proof_request};

pub async fn submit_handler<
    T: Transport + Clone,
    P: Provider<T> + Clone,
    I: ProvingSystemInformation + Clone,
>(
    app_state: State<AppState<T, P, ProofRequest<I>>>,
    Json(request): Json<ProofRequest<I>>,
) -> Result<impl IntoResponse, impl IntoResponse> {
    let minimum_allowed_proving_time = app_state.minimum_allowed_proving_time();
    let maximum_allowed_start_delay = app_state.maximum_allowed_start_delay();
    let maximum_allowed_stake = app_state.maximum_allowed_stake();
    let timeout = app_state.validation_timeout_seconds();

    log::info!("Validating proof request");
    match validate_proof_request(
        &request,
        &app_state,
        minimum_allowed_proving_time,
        maximum_allowed_start_delay,
        maximum_allowed_stake,
        timeout,
    )
    .await
    {
        Ok(()) => {
            log::debug!("Validation successful, attempting to broadcast");
            match app_state.subscription_manager().broadcast(request) {
                Ok(recv_count) => {
                    log::info!(
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
                    log::debug!("No active subscribers to receive the broadcast");
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
            log::warn!("Validation failed: {:?}", e);
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
