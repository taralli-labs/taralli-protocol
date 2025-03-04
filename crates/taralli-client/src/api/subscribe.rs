use std::pin::Pin;

use futures::{Stream, StreamExt};
use taralli_primitives::{
    env::Environment,
    intents::request::ComputeRequest,
    systems::{SystemId, SystemParams},
};
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use tungstenite::{handshake::client::generate_key, Message};
use url::Url;

use crate::{
    api::compression::decompress_intent,
    error::{ClientError, Result},
};

// type alias for stream of compute requests returned by the protocol server
pub type RequestStream = Pin<Box<dyn Stream<Item = Result<ComputeRequest<SystemParams>>> + Send>>;

pub struct SubscribeApiClient {
    server_url: Url,
    api_key: String,
}

impl SubscribeApiClient {
    pub fn new(server_url: Url) -> Self {
        let mut api_key = String::new();
        if Environment::from_env_var() == Environment::Production {
            api_key = std::env::var("API_KEY").expect("API_KEY env variable is not set");
        }

        Self {
            api_key,
            server_url,
        }
    }

    pub async fn subscribe_to_markets(&self, system_ids: &[SystemId]) -> Result<RequestStream> {
        let mut url = self
            .server_url
            .join("/subscribe")
            .map_err(|e| ClientError::ServerSubscriptionError(e.to_string()))?;

        let scheme = url.scheme().to_string();

        let new_scheme = match scheme.as_str() {
            "http" => "ws",
            "https" => "wss",
            other => other,
        };
        url.set_scheme(new_scheme).map_err(|_| {
            ClientError::ServerSubscriptionError("Invalid WebSocket scheme".to_string())
        })?;

        let query = system_ids
            .iter()
            .map(|id| id.as_str().to_string())
            .collect::<Vec<_>>()
            .join(",");

        let uri = format!("/subscribe?system_ids={}", query);

        let ws_url = url
            .join(&uri)
            .map_err(|e| ClientError::ServerSubscriptionError(e.to_string()))?
            .to_string();

        // tracing::info!("Connecting to WebSocket: {ws_url}");

        let request = tungstenite::http::Request::builder()
            .uri(&ws_url)
            .header(
                "Host",
                url.host_str().ok_or_else(|| {
                    ClientError::ServerSubscriptionError("Invalid WebSocket host".to_string())
                })?,
            )
            .header("x-api-key", self.api_key.clone())
            .header("Sec-WebSocket-Key", generate_key())
            .header("Sec-WebSocket-Version", "13")
            .header("Connection", "Upgrade")
            .header("Upgrade", "websocket")
            .body(())
            .map_err(|e| {
                ClientError::ServerSubscriptionError(format!("Request build error: {e}"))
            })?;

        tracing::info!("Connecting to WebSocket: {ws_url}");
        let (ws_stream, _resp) = connect_async(request).await.map_err(|e| {
            ClientError::ServerSubscriptionError(format!("WebSocket connect error: {e}"))
        })?;

        Self::get_intent_stream(ws_stream).await
    }

    /// Parse a WebSocket stream into a stream of requests
    /// # Arguments
    /// * `stream` - The WebSocket stream
    /// # Returns
    /// * A stream of requests
    /// # Errors
    /// * If the WebSocket stream is unavailable
    /// * If the WebSocket stream is not a Brotli-compressed message
    /// * If the WebSocket stream is not a valid JSON message
    /// * If the WebSocket stream is not a valid request
    async fn get_intent_stream(
        stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
    ) -> Result<RequestStream> {
        let (_write, read) = stream.split();

        let parsed_stream = read.filter_map(|message_result| async {
            match message_result {
                // This is the only case we care about.
                // We expect the server to send us Brotli-compressed binary messages.
                Ok(Message::Binary(compressed_bytes)) => {
                    tracing::info!("Received Brotli-compressed binary message");

                    // First we need to decompress the bytes.
                    let decompressed_bytes =
                        match decompress_intent(compressed_bytes.to_vec()).await {
                            Ok(decompressed) => decompressed,
                            Err(e) => {
                                tracing::error!("Failed to decompress WebSocket data: {:?}", e);
                                return Some(Err(ClientError::IntentParsingError(format!(
                                    "Failed to decompress WebSocket data: {e}"
                                ))));
                            }
                        };

                    // Then deserialize the JSON from decompressed bytes
                    match serde_json::from_slice::<ComputeRequest<SystemParams>>(
                        &decompressed_bytes,
                    ) {
                        Ok(parsed) => Some(Ok(parsed)),
                        Err(e) => Some(Err(ClientError::IntentParsingError(format!(
                            "JSON parse error after decompression: {e}"
                        )))),
                    }
                }
                Ok(Message::Frame(_)) => {
                    tracing::info!("Received unexpected frame message.");
                    None
                }
                Ok(Message::Text(_)) => {
                    tracing::info!("Received unexpected text message instead of binary.");
                    None
                }
                Ok(Message::Close(_)) => {
                    tracing::info!("WebSocket closed by server.");
                    None
                }
                Ok(Message::Ping(_) | Message::Pong(_)) => None,
                Err(e) => Some(Err(ClientError::IntentParsingError(format!(
                    "WebSocket error: {e}"
                )))),
            }
        });
        Ok(Box::pin(parsed_stream))
    }
}
