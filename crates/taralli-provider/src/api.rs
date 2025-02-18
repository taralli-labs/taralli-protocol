use async_compression::tokio::bufread::BrotliDecoder;
use futures::{Stream, StreamExt};
use taralli_primitives::RequestCompressed;
use tokio::net::TcpStream;

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
}

// type alias for SSE stream returned by the protocol server
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
    /// * `stream` - The WebSocket stream
    /// # Returns
    /// * A stream of requests
    /// # Errors
    /// * If the WebSocket stream is unavailable
    /// * If the WebSocket stream is not a Brotli-compressed message
    /// * If the WebSocket stream is not a valid JSON message
    /// * If the WebSocket stream is not a valid request
    async fn get_stream(
        stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
    ) -> Result<RequestStream> {
        let (_write, read) = stream.split();

        let parsed_stream = read.filter_map(|message_result| async {
            match message_result {
                // This is the only case we care about.
                Ok(Message::Binary(bytes)) => {
                    // First we deserialize the data sent via the WebSocket.
                    let request_compressed: RequestCompressed = match bincode::deserialize(&bytes) {
                        Ok(rc) => rc,
                        Err(e) => {
                            tracing::info!("Couldn't deserialize data from WebSocket");
                            return Some(Err(ProviderError::RequestParsingError(format!(
                                "Failed to deserialize WebSocket data: {:?}",
                                e
                            ))));
                        }
                    };

                    // Then, we need to decompress the proving system information.
                    let proving_system_information: ProvingSystemParams =
                        match ProviderApi::decompress_brotli(
                            request_compressed.proving_system_information,
                        )
                        .await
                        {
                            Ok(decompressed) => decompressed,
                            Err(e) => {
                                tracing::error!(
                                    "Failed to decompress proving system information: {:?}",
                                    e
                                );
                                return Some(Err(ProviderError::RequestParsingError(format!(
                                    "Failed to decompress proving system information data: {e}"
                                ))));
                            }
                        };

                    Some(Ok(Request::<ProvingSystemParams> {
                        proving_system_id: request_compressed.proving_system_id,
                        proving_system_information,
                        onchain_proof_request: request_compressed.onchain_proof_request,
                        signature: request_compressed.signature,
                    }))
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
                Err(e) => Some(Err(ProviderError::RequestParsingError(format!(
                    "WebSocket error: {e}"
                )))),
            }
        });
        Ok(Box::pin(parsed_stream))
    }

    pub async fn subscribe_to_markets(&self) -> Result<RequestStream> {
        let mut url = self
            .server_url
            .join("/subscribe")
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

        let ws_url = url
            .join("subscribe")
            .map_err(|e| ProviderError::ServerSubscriptionError(e.to_string()))?
            .to_string();

        tracing::info!("Connecting to WebSocket: {ws_url}");

        let request = tungstenite::http::Request::builder()
            .uri(ws_url)
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

        Self::get_stream(ws_stream).await
    }
}
