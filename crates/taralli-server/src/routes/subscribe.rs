use std::sync::Arc;

use alloy::{providers::Provider, transports::Transport};
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    response::IntoResponse,
};
use futures::{
    stream::{select_all, StreamExt},
    SinkExt,
};
use serde::Deserialize;
use taralli_primitives::systems::SystemId;
use tokio_stream::wrappers::BroadcastStream;

use crate::error::{Result, ServerError};
use crate::state::request::RequestState;

#[derive(Debug, Deserialize)]
pub struct SubscribeQuery {
    pub system_ids: String,
}

/// WebSocket subscription handler that upgrades the connection to a WebSocket session.
///
/// This function acts as an entry point for WebSocket clients that want to subscribe
/// to proof-related updates. It upgrades the connection and delegates the handling to
/// `websocket_subscribe`.
///
/// # Parameters
/// - `ws`: The WebSocket upgrade request from the client.
/// - `app_state`: Shared application state, containing the subscription manager.
///
/// # Returns
/// An `IntoResponse` that upgrades the HTTP connection to a WebSocket session, which is needed since we expose the WebSocket endpoint as an HTTP route.
pub async fn subscribe_handler<T: Transport + Clone + 'static, P: Provider<T> + Clone + 'static>(
    ws: WebSocketUpgrade,
    Query(params): Query<SubscribeQuery>,
    State(app_state): State<RequestState<T, P>>,
) -> Result<impl IntoResponse> {
    tracing::info!("subscribe called");
    // parse submitted IDs
    let ids = params.system_ids.split(',').collect::<Vec<&str>>();
    let mut invalid_ids = Vec::new();
    let mut valid_ids = Vec::new();
    for id_str in ids {
        match SystemId::try_from(id_str) {
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

    Ok(ws.on_upgrade(move |socket| websocket_subscribe(socket, Arc::new(app_state), valid_ids)))
}

/// Handles an active WebSocket session, streaming messages from the subscription system.
///
/// This function listens to the broadcast stream from the `subscription_manager` and sends
/// new messages to the connected WebSocket client. If an error occurs while sending,
/// the connection is closed.
///
/// # Parameters
/// - `socket`: The WebSocket connection.
/// - `app_state`: Shared application state, containing the subscription manager.
async fn websocket_subscribe<T: Transport + Clone, P: Provider<T> + Clone>(
    socket: WebSocket,
    app_state: Arc<RequestState<T, P>>,
    system_ids: Vec<SystemId>,
) {
    // Register a new subscription. In other words, create a new receiver for the broadcasted proofs.
    // let subscription = app_state.subscription_manager(). add_subscription();
    tracing::info!("Valid IDs submitted, creating ws stream");

    let receivers = app_state
        .subscription_manager()
        .subscribe_to_ids(&system_ids)
        .await;

    // Create a broadcast stream from the subscription receiver.
    // let mut broadcast_stream = BroadcastStream::new(subscription);

    // Convert receivers to SSE streams
    let streams = receivers.into_iter().map(|rx| {
        BroadcastStream::new(rx).map(|result| result.map_err(|e| axum::Error::new(e.to_string())))
    });

    let mut meta_stream = select_all(streams);

    // Split the WebSocket into sender/receiver so we can handle them separately
    let (mut ws_sender, mut ws_receiver) = socket.split();

    tracing::info!("stream created, initiating websocket loop");

    // Use a `tokio::select!` loop to handle both reading and writing since we're in an async context.
    loop {
        tokio::select! {
            // Outbound: messages from broadcast_stream => client
            maybe_broadcast = meta_stream.next() => {
                match maybe_broadcast {
                    Some(Ok(bytes)) => {
                        // Try sending a binary message to the client
                        if let Err(e) = ws_sender.send(Message::Binary(bytes)).await {
                            tracing::error!("Failed to send WebSocket message: {:?}", e);
                            break;
                        }
                    }
                    Some(Err(e)) => {
                        tracing::error!("Broadcast stream error: {:?}", e);
                        break;
                    }
                    None => {
                        // The broadcast_stream ended (channel closed, etc.)
                        break;
                    }
                }
            },

            // Inbound: messages from client => (potentially) the server
            // There's not a lot we want to do with incoming messages in this case, despite the usage of websockets
            // this is (mostly) a one-way communication channel.
            // We need to handle the disconnect, otherwise we'll have dangling connections.
            maybe_incoming = ws_receiver.next() => {
                match maybe_incoming {
                    Some(Ok(Message::Close(_))) => {
                        tracing::info!("Client sent Close");
                        break;
                    }
                    Some(Ok(Message::Ping(_)) | Ok(Message::Pong(_))) => {
                        // Received a ping or pong, no need to do anything
                    }
                    Some(Err(e)) => {
                        tracing::error!("Read error from client: {:?}", e);
                        break;
                    }
                    // We're not interested in other message types
                    Some(_) => {
                        tracing::info!("Received unexpected message from client");
                    }
                    None => {
                        // Client disconnected cleanly
                        break;
                    }
                }
            }
        }
    }
}
