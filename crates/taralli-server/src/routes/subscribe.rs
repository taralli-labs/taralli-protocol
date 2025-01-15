use alloy::{providers::Provider, transports::Transport};
use axum::{
    extract::State,
    response::sse::{Event, Sse},
};
use futures::stream::StreamExt;
use taralli_primitives::taralli_systems::traits::ProvingSystemInformation;
use taralli_primitives::ProofRequest;
use tokio_stream::wrappers::BroadcastStream;

use crate::app_state::AppState;

pub async fn subscribe_handler<
    T: Transport + Clone,
    P: Provider<T> + Clone,
    I: ProvingSystemInformation + Clone,
>(
    app_state: State<AppState<T, P, ProofRequest<I>>>,
) -> Sse<impl futures::Stream<Item = Result<Event, axum::Error>>> {
    let recv_new = app_state.subscription_manager().add_subscription();
    tracing::info!(
        "subscription has been added, receiver count: {}",
        app_state.subscription_manager().active_subscriptions()
    );
    Sse::new(BroadcastStream::new(recv_new).map(|result| {
        result.map_err(axum::Error::new).and_then(|proof_req| {
            Event::default()
                .json_data(proof_req)
                .map_err(axum::Error::new)
        })
    }))
}
