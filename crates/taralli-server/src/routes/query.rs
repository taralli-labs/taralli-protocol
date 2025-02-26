use crate::{
    error::{Result, ServerError},
    state::offer::OfferState,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde_json::json;
use taralli_primitives::alloy::{providers::Provider, transports::Transport};
use taralli_primitives::systems::SystemId;

pub async fn get_active_intents_by_id_handler<T: Transport + Clone, P: Provider<T> + Clone>(
    State(app_state): State<OfferState<T, P>>,
    Path(system_id): Path<String>,
) -> Result<(StatusCode, Json<serde_json::Value>)> {
    let system_id = SystemId::try_from(system_id.as_str())
        .map_err(|e| ServerError::QueryError(e.to_string()))?;

    let intents = app_state
        .intent_db()
        .get_active_intents_by_id(system_id)
        .await?;

    Ok((StatusCode::OK, Json(json!({ "intents": intents }))))
}
