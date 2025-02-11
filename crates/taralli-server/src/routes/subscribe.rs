use axum::{
    extract::{Query, State},
    response::sse::{Event, KeepAlive, Sse},
};
use futures::stream::{select_all, StreamExt};
use serde::Deserialize;
use taralli_primitives::alloy::{providers::Provider, transports::Transport};
use taralli_primitives::systems::ProvingSystemId;
use tokio_stream::wrappers::BroadcastStream;

use crate::error::{Result, ServerError};
use crate::state::request::RequestState;

#[derive(Debug, Deserialize)]
pub struct SubscribeQuery {
    pub system_ids: String,
}

pub async fn subscribe_handler<T, P>(
    State(app_state): State<RequestState<T, P>>,
    Query(params): Query<SubscribeQuery>,
) -> Result<Sse<impl futures::Stream<Item = core::result::Result<Event, axum::Error>>>>
where
    T: Transport + Clone,
    P: Provider<T> + Clone,
{
    let ids = params.system_ids.split(',').collect::<Vec<&str>>();
    let mut invalid_ids = Vec::new();
    let mut valid_ids = Vec::new();
    for id_str in ids {
        match ProvingSystemId::try_from(id_str) {
            Ok(id) => valid_ids.push(id),
            Err(_) => invalid_ids.push(id_str),
        }
    }

    // If any invalid IDs were found, return error with details
    if !invalid_ids.is_empty() {
        return Err(ServerError::SystemIdError(format!(
            "Invalid proving system IDs: {}",
            invalid_ids.join(", ")
        )));
    }

    // Get broadcast receivers for each valid system ID
    let receivers = app_state
        .subscription_manager()
        .subscribe_to_ids(&valid_ids)
        .await;

    // Convert receivers to SSE streams
    let streams = receivers.into_iter().map(|rx| {
        BroadcastStream::new(rx).map(|result| {
            result
                .map_err(|e| axum::Error::new(e.to_string()))
                .and_then(|request| {
                    Event::default()
                        .json_data(request)
                        .map_err(|e| axum::Error::new(e.to_string()))
                })
        })
    });

    // Merge all streams and create SSE
    Ok(Sse::new(select_all(streams))
        .keep_alive(KeepAlive::new().interval(std::time::Duration::from_secs(15))))
}
