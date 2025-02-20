use async_compression::tokio::bufread::BrotliDecoder;
use futures::{Stream, StreamExt};
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client,
};
use std::io::Write;
use std::pin::Pin;
use taralli_primitives::intents::ComputeIntent;
use taralli_primitives::{
    env::Environment,
    intents::ComputeRequest,
    systems::{SystemId, SystemParams},
};
use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use tungstenite::handshake::client::generate_key;
use tungstenite::Message;
use url::Url;

use crate::error::{ClientError, Result};

pub struct ApiClient {
    api_key: String,
    client: Client,
    server_url: Url,
}

// type alias for stream of compute requests returned by the protocol server
pub type RequestStream = Pin<Box<dyn Stream<Item = Result<ComputeRequest<SystemParams>>> + Send>>;

impl ApiClient {
    pub fn new(server_url: Url) -> Self {
        let mut headers = HeaderMap::new();
        headers.insert("Content-Type", HeaderValue::from_static("application/json"));
        headers.insert("Content-Encoding", HeaderValue::from_static("br"));

        let mut api_key = String::new();
        if Environment::from_env_var() == Environment::Production {
            api_key = std::env::var("API_KEY").expect("API_KEY env variable is not set");
        }

        Self {
            api_key,
            client: Client::builder()
                .default_headers(headers)
                .build()
                .expect("Failed to build reqwest client"),
            server_url,
        }
    }

    /// Compresses the intent payload using Brotli compression
    /// and returns the compressed payload as a byte vector
    /// # Arguments
    /// * `request` - The intent to be compressed
    /// # Returns
    /// * A byte vector containing the compressed payload
    /// # Details
    /// The compression level, buffer size, and window size are configurable
    /// via the environment variables.
    /// Furthermore, we chose to instantiate a new compressor for each intent
    /// if the need to submit multiple intent concurrently arises.
    fn compress_intent<I: ComputeIntent>(&self, intent: I) -> Result<Vec<u8>> {
        // We opt for some default values that may be reasonable for the general use case.
        let mut brotli_encoder = brotli::CompressorWriter::new(
            Vec::new(),
            std::env::var("BROTLI_BUFFER_SIZE")
                .unwrap_or_else(|_| "0".to_string())
                .parse::<usize>()
                .unwrap_or(0),
            std::env::var("BROTLI_COMPRESSION_LEVEL")
                .unwrap_or_else(|_| "7".to_string())
                .parse::<u32>()
                .unwrap_or(7),
            std::env::var("BROTLI_WINDOW_SIZE")
                .unwrap_or_else(|_| "24".to_string())
                .parse::<u32>()
                .unwrap_or(24),
        );

        let payload = serde_json::to_string(&intent)
            .map_err(|e| ClientError::IntentSubmissionFailed(e.to_string()))?;

        brotli_encoder
            .write_all(&payload.as_bytes())
            .map_err(|e| ClientError::IntentSubmissionFailed(e.to_string()))?;

        Ok(brotli_encoder.into_inner())
    }

    /// Decompress a Brotli-compressed byte vector
    /// # Arguments
    /// * `compressed_bytes` - The Brotli-compressed byte vector
    /// # Returns
    /// * A byte vector containing the decompressed data
    async fn decompress_intent(compressed_bytes: Vec<u8>) -> Result<Vec<u8>> {
        let mut decoder = BrotliDecoder::new(tokio::io::BufReader::new(&compressed_bytes[..]));
        let mut decompressed = Vec::new();
        decoder
            .read_to_end(&mut decompressed)
            .await
            .map_err(|e| ClientError::IntentDecompressionFailed(e.to_string()))?;
        Ok(decompressed)
    }

    pub async fn submit_intent<I: ComputeIntent>(&self, intent: I) -> Result<reqwest::Response> {
        let url = self
            .server_url
            .join("/submit")
            .map_err(|e| ClientError::ServerUrlParsingError(e.to_string()))?;

        let compressed_payload = self.compress_intent(intent)?;

        let response = self
            .client
            .post(url)
            .body(compressed_payload)
            .send()
            .await
            .map_err(|e| ClientError::ServerRequestError(e.to_string()))?;
        Ok(response)
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
                        match ApiClient::decompress_intent(compressed_bytes.to_vec()).await {
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
}
