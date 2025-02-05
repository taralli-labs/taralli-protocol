use axum::{
    extract::State,
    response::sse::{Event, Sse},
};
use futures::stream::StreamExt;
use taralli_primitives::alloy::{providers::Provider, transports::Transport};
use tokio_stream::wrappers::BroadcastStream;

use crate::state::request::RequestState;

pub async fn subscribe_handler<T: Transport + Clone, P: Provider<T> + Clone>(
    State(app_state): State<RequestState<T, P>>,
) -> Sse<impl futures::Stream<Item = Result<Event, axum::Error>>> {
    let recv_new = app_state.subscription_manager().add_subscription();
    tracing::info!(
        "subscription has been added, receiver count: {}",
        app_state.subscription_manager().active_subscriptions()
    );
    Sse::new(BroadcastStream::new(recv_new).map(|result| {
        result.map_err(axum::Error::new).and_then(|request| {
            Event::default()
                .json_data(request)
                .map_err(axum::Error::new)
        })
    }))
}
