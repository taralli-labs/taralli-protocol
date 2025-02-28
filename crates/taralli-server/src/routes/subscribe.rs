use alloy::{providers::Provider, transports::Transport};
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    response::IntoResponse,
};
use futures::{stream::StreamExt, SinkExt};
use serde::Deserialize;
use std::sync::Arc;
use taralli_primitives::systems::ProvingSystemMask;
use tokio_stream::wrappers::BroadcastStream;

use crate::app_state::AppState;

#[derive(Debug, Deserialize)]
pub struct SubscribeArgs {
    pub subscribed_to: Option<ProvingSystemMask>,
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
pub async fn websocket_subscribe_handler<
    T: Transport + Clone + 'static,
    P: Provider<T> + Clone + 'static,
>(
    ws: WebSocketUpgrade,
    State(app_state): State<AppState<T, P>>,
    Query(args): Query<SubscribeArgs>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| {
        websocket_subscribe(socket, Arc::new(app_state), args.subscribed_to)
    })
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
    app_state: Arc<AppState<T, P>>,
    subscribed_to: Option<ProvingSystemMask>,
) {
    // Register a new subscription. In other words, create a new receiver for the broadcasted proofs.
    let subscription = app_state.subscription_manager().add_subscription();
    tracing::info!(
        "Subscription added, active subscriptions: {}",
        app_state.subscription_manager().active_subscriptions()
    );

    // Create a broadcast stream from the subscription receiver.
    let mut broadcast_stream = BroadcastStream::new(subscription);

    // Split the WebSocket into sender/receiver so we can handle them separately
    let (mut ws_sender, mut ws_receiver) = socket.split();
    // Use a `tokio::select!` loop to handle both reading and writing since we're in an async context.
    loop {
        tokio::select! {
            // Outbound: messages from broadcast_stream => client
            maybe_broadcast = broadcast_stream.next() => {
                match maybe_broadcast {
                    Some(Ok(message)) => {
                        let bytes = message.content;
                        let message_proving_system_id = message.subscribed_to;
                        // Check if the message is for the subscribed proving system
                        // If no proving system is specified, default to 1, client is subscribed to all proving systems.
                        if message_proving_system_id & subscribed_to.unwrap_or(1) == 0 {
                            continue;
                        }
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

            // Inbound: messages from client => server
            // There's not a lot we want to do with incoming messages in this case, despite the usage of websockets
            // this is (mostly) a one-way communication channel.
            maybe_incoming = ws_receiver.next() => {
                match maybe_incoming {
                    Some(Ok(Message::Close(mut message))) => {
                        tracing::info!("Client sent Close: {:?}", message.take());
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
