use async_compression::tokio::bufread::BrotliDecoder;
use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, Stream, StreamExt};
use std::time::Duration;
use taralli_primitives::RequestCompressed;
use tokio::net::TcpStream;
use tokio::signal;
use tokio::time::timeout;
use tungstenite::protocol::frame::coding::CloseCode;
use tungstenite::protocol::CloseFrame;

use std::pin::Pin;
use taralli_primitives::common::types::Environment;
use taralli_primitives::{systems::ProvingSystemParams, Request};
use tokio::io::AsyncReadExt;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use tungstenite::handshake::client::generate_key;
use tungstenite::Message;
use url::Url;

use crate::{
    config::ApiConfig,
    error::{ProviderError, Result},
};

pub struct ProviderApi {
    api_key: String,
    server_url: Url,
    subscribed_to: u8,
}

// type alias for WebSocket stream returned by the protocol server
pub type RequestStream = Pin<Box<dyn Stream<Item = Result<Request<ProvingSystemParams>>> + Send>>;

impl ProviderApi {
    pub fn new(config: ApiConfig) -> Self {
        let mut api_key = String::new();

        if Environment::from_env_var() == Environment::Production {
            api_key = std::env::var("API_KEY").expect("API_KEY env variable is not set");
        }

        Self {
            api_key,
            server_url: config.server_url,
            subscribed_to: config.subscribed_to,
        }
    }

    /// Decompress a Brotli-compressed byte vector
    /// # Arguments
    /// * `compressed_bytes` - The Brotli-compressed byte vector
    /// # Returns
    /// * A byte vector containing the decompressed data
    async fn decompress_brotli(
        compressed_bytes: Vec<u8>,
    ) -> std::result::Result<ProvingSystemParams, std::io::Error> {
        let mut decoder = BrotliDecoder::new(tokio::io::BufReader::new(&compressed_bytes[..]));
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed).await?;
        let params = serde_json::from_slice(&decompressed)?;
        Ok(params)
    }

    /// Parse a WebSocket stream into a stream of requests
    /// # Arguments
    /// * `listener` - The listener object associated to the websocket stream.
    /// * `shutdown_receiver` - The receiving side of IPC communication linking WebSocket streams.
    /// # Returns
    /// * A stream of requests.
    /// # Errors
    /// * If the WebSocket stream is unavailable
    /// * If the WebSocket stream isn't properly serialized
    /// * If the WebSocket stream is not a Brotli-compressed message
    /// * If the WebSocket stream is not a valid JSON message
    /// * If the WebSocket stream is not a valid request
    /// * If the WebSocket stream has unexpected message types
    async fn get_stream_with_shutdown(
        listener: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
        shutdown_receiver: tokio::sync::oneshot::Receiver<()>,
    ) -> Result<RequestStream> {
        // Start a `stream::unfold`, which keeps passing listener and shutdown_receiver to next iterations,
        // whilst yielding `Request<ProvingSystemParams>` at the end of each iteration.
        let parsed_stream = futures::stream::unfold(
        (listener, shutdown_receiver),
        |(mut listener, mut shutdown_receiver)| async move {
            // We will asynchronously wait for a websocket message or a shutdown signal. 
            tokio::select! {
                // Wait for next WebSocket message
                maybe_message = listener.next() => {
                    match maybe_message {
                        // This is the only case that yields action from us.
                        // We expect the server to send us serialized, Brotli-compressed, binary messages.
                        Some(Ok(Message::Binary(bytes))) => {
                            // First we deserialize the data sent via the WebSocket.
                            let request_compressed: RequestCompressed = match bincode::deserialize(&bytes) {
                                Ok(rc) => rc,
                                Err(e) => {
                                    let err = Err(crate::ProviderError::RequestParsingError(
                                        format!("Failed to deserialize WebSocket data: {:?}", e)
                                    ));
                                    // Yield an error item but continue the stream
                                    return Some((err, (listener, shutdown_receiver)));
                                }
                            };

                            // Then, we need to decompress the proving system information.
                            let proving_system_information = match super::ProviderApi::decompress_brotli(
                                request_compressed.proving_system_information
                            ).await {
                                Ok(decompressed) => decompressed,
                                Err(e) => {
                                    let err = Err(crate::ProviderError::RequestParsingError(
                                        format!("Failed to decompress proving system data: {e}")
                                    ));
                                    return Some((err, (listener, shutdown_receiver)));
                                }
                            };

                            // Create the final Request object
                            let request = crate::Request::<ProvingSystemParams> {
                                proving_system_id: request_compressed.proving_system_id,
                                proving_system_information,
                                onchain_proof_request: request_compressed.onchain_proof_request,
                                signature: request_compressed.signature,
                            };

                            // Yield a successful `Ok(...)` item, continuing the stream
                            Some((Ok(request), (listener, shutdown_receiver)))
                        }
                        // If the server sends a Close frame, or a ping/pong/frame/text,
                        // we either log or yield an Err. For brevity, we skip or return `None`.
                        Some(Ok(Message::Close(cf))) => {
                            tracing::info!("WebSocket closed by server: {:?}", cf);
                            None // End the stream
                        }
                        Some(Ok(_other)) => {
                            tracing::info!("Ignoring unexpected message type.");
                            // We continue by yielding an error upstream.
                            let err = Err(crate::ProviderError::RequestParsingError(
                                "Ignoring unexpected message type".to_string()
                            ));
                            Some((err, (listener, shutdown_receiver)))
                        }
                        // If an actual error occurs, end the stream
                        Some(Err(e)) => {
                            tracing::error!("WebSocket error: {:?}", e);
                            None
                        }
                        // The underlying stream ended
                        None => {
                            tracing::info!("WebSocket stream ended (None).");
                            None
                        }
                    }
                },
                // Handle shutdown signal from oneshot
                _ = &mut shutdown_receiver => {
                    tracing::info!("Request stream shutting down due to signal.");
                    None // end the stream
                }
            }
        }
    )
    // `unfold` yields `Some((Item, State))`, but we want only the `Item`.
    .filter_map(|item| async move {
        // item is `(Result<Request<ProvingSystemParams>>)`
        Some(item)
    });

        Ok(Box::pin(parsed_stream))
    }

    pub async fn subscribe_to_markets(&self) -> Result<RequestStream> {
        let mut url = self
            .server_url
            .join(format!("/subscribe?subscribed_to={}", self.subscribed_to).as_str())
            .map_err(|e| ProviderError::ServerSubscriptionError(e.to_string()))?;

        let scheme = url.scheme().to_string();

        let new_scheme = match scheme.as_str() {
            "http" => "ws",
            "https" => "wss",
            other => other,
        };
        url.set_scheme(new_scheme).map_err(|_| {
            ProviderError::ServerSubscriptionError("Invalid WebSocket scheme".to_string())
        })?;

        tracing::info!("Connecting to WebSocket: {url}");

        let request = tungstenite::http::Request::builder()
            .uri(url.as_str())
            .header(
                "Host",
                url.host_str().ok_or_else(|| {
                    ProviderError::ServerSubscriptionError("Invalid WebSocket host".to_string())
                })?,
            )
            .header("x-api-key", self.api_key.clone())
            .header("Sec-WebSocket-Key", generate_key())
            .header("Sec-WebSocket-Version", "13")
            .header("Connection", "Upgrade")
            .header("Upgrade", "websocket")
            .body(())
            .map_err(|e| {
                ProviderError::ServerSubscriptionError(format!("Request build error: {e}"))
            })?;

        let (ws_stream, _resp) = connect_async(request).await.map_err(|e| {
            ProviderError::ServerSubscriptionError(format!("WebSocket connect error: {e}"))
        })?;

        // Split the websocket since we're only receiving data on this client side.
        // Therefore, sender side will have the purpose of sending packets to close the connection.
        // Hence it'll be within a separate thread.
        let (ws_sender, ws_listener) = ws_stream.split();

        // Create oneshot channels so sender and receiver from websocket can both be closed appropriately.
        let (shutdown_sender, shutdown_receiver) = tokio::sync::oneshot::channel::<()>();
        let (cleanup_sender, cleanup_receiver) = tokio::sync::oneshot::channel::<()>(); // <-- new!

        // Spawn the shutdown handler with the sender half.
        tokio::spawn(Self::wait_for_shutdown_and_close_ws(
            ws_sender,
            shutdown_sender,
            cleanup_receiver,
        ));

        // Create a stream that processes messages until shutdown is received
        let parsed_stream = Self::get_stream_with_shutdown(ws_listener, shutdown_receiver).await?;

        let wrapped_stream = CleanupStream {
            inner: parsed_stream,
            cleanup_sender: Some(cleanup_sender), // <-- new!
        };

        Ok(Box::pin(wrapped_stream))
    }

    /// Waits for a shutdown and attempts to gracefully close the WebSocket connection.
    ///
    /// This function listens to shutdown signal, which, if received, will attempt to send
    /// a `Close` message to the server, as well as oneshot communicate with the receiver side
    /// of the split WebSocket to close the stream.
    ///
    /// # Parameters
    /// - `ws_sender`: The sending side of the WebSocket.
    /// - `shutdown_sender`: The sending side of IPC communication linking WebSocket streams.
    /// - `cleanup_receiver`: The receiving side of IPC communication, triggered when the stream is dropped.
    pub async fn wait_for_shutdown_and_close_ws(
        mut ws_sender: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
        shutdown_sender: tokio::sync::oneshot::Sender<()>,
        mut cleanup_receiver: tokio::sync::oneshot::Receiver<()>,
    ) {
        let mut term_signal =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("Failed to create SIGTERM handler");

        // Wait for a shutdown signal
        tokio::select! {
            _ = signal::ctrl_c() => tracing::warn!("Received SIGINT (Ctrl+C), shutting down."),
            _ = term_signal.recv() => tracing::warn!("Received SIGTERM, shutting down."),
            _ = &mut cleanup_receiver => {
                tracing::info!("Cleanup triggered (stream dropped), closing connection.");
            }
        }

        // Send a Close frame before exiting
        tracing::info!("Closing WebSocket connection cleanly...");

        let close_frame = CloseFrame {
            code: CloseCode::Normal,
            reason: "Client shutting down".into(),
        };

        if let Err(e) = ws_sender.send(Message::Close(Some(close_frame))).await {
            tracing::warn!("Failed to send close frame: {}", e);
        }

        // Signal the reader that we're shutting down
        let _ = shutdown_sender.send(());

        // Delay to allow the close frame to be sent
        let timeout_duration = Duration::from_millis(1000);
        let close_ack = timeout(timeout_duration, ws_sender.close()).await;

        match close_ack {
            Ok(Ok(())) => tracing::info!("Server acknowledged WebSocket closure."),
            Ok(Err(e)) => tracing::warn!("Error waiting for server Close frame: {:?}", e),
            Err(_) => tracing::warn!("Timeout waiting for server Close frame."),
        }

        tracing::info!("WebSocket closed.");
    }
}

/// Wrapper around the RequestStream type.
/// The intent here is to implement a custom `Drop` so we can set the closing of WebSocket conns.
pub struct CleanupStream {
    inner: RequestStream,
    cleanup_sender: Option<tokio::sync::oneshot::Sender<()>>,
}

impl Stream for CleanupStream {
    type Item = Result<Request<ProvingSystemParams>>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        Pin::new(&mut self.inner).poll_next(cx)
    }
}

impl Drop for CleanupStream {
    fn drop(&mut self) {
        // Start closing the websocket.
        if let Some(sender) = self.cleanup_sender.take() {
            let _ = sender.send(());
        }
    }
}
