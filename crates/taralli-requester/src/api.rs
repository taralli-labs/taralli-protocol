use reqwest::{
    header::{HeaderMap, HeaderValue},
    multipart, Client,
};
use serde_json::json;
use std::io::Write;
use taralli_primitives::common::types::Environment;
use taralli_primitives::{systems::ProvingSystemParams, Request};
use url::Url;

use crate::error::{RequesterError, Result};

pub struct RequesterApi {
    client: Client,
    server_url: Url,
}

impl RequesterApi {
    pub fn new(server_url: Url) -> Self {
        let mut headers = HeaderMap::new();
        headers.insert("Content-Type", HeaderValue::from_static("application/json"));
        headers.insert("Content-Encoding", HeaderValue::from_static("br"));

        if Environment::from_env_var() == Environment::Production {
            let api_key = std::env::var("API_KEY").expect("API_KEY env variable is not set");
            headers.insert(
                "x-api-key",
                HeaderValue::from_str(&api_key).expect("API_KEY is invalid as a header"),
            );
        }

        Self {
            client: Client::builder()
                .default_headers(headers)
                .build()
                .expect("Failed to build reqwest client"),
            server_url,
        }
    }

    /// Compresses the proving system information using Brotli and returns the it as a byte vector
    /// # Arguments
    /// * `proving_system_information` - The proving system information to be compressed
    /// # Returns
    /// * A byte vector containing the compressed payloa
    /// # Details
    /// The compression level, buffer size, and window size are configurable
    /// via the environment variables.
    /// Furthermore, we chose to instantiate a new compressor for each request
    /// if the need to submit multiple requests concurrently arises.
    fn compress_proving_system_information(
        &self,
        proving_system_information: ProvingSystemParams,
    ) -> Result<Vec<u8>> {
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

        let payload = serde_json::to_string(&proving_system_information)
            .map_err(|e| RequesterError::RequestSubmissionFailed(e.to_string()))?;

        brotli_encoder
            .write_all(payload.as_bytes())
            .map_err(|e| RequesterError::RequestSubmissionFailed(e.to_string()))?;

        Ok(brotli_encoder.into_inner())
    }

    /// Returns Multipart request Form with two parts: `ProvingSystemParams` as a `application/octet-stream` and remaining
    /// fields as `application/json`.
    fn build_multipart(&self, request: Request<ProvingSystemParams>) -> Result<multipart::Form> {
        let partial_request = json!({
            "proving_system_id": request.proving_system_id,
            "onchain_proof_request": request.onchain_proof_request,
            "signature": request.signature,
        });
        let partial_request_string = serde_json::to_string(&partial_request)
            .map_err(|e| RequesterError::RequestSubmissionFailed(e.to_string()))?;
        let partial_request_part = multipart::Part::text(partial_request_string);

        let compressed =
            self.compress_proving_system_information(request.proving_system_information)?;
        let compressed_part = multipart::Part::bytes(compressed);

        let form = multipart::Form::new()
            .part("partial_request", partial_request_part)
            .part("proving_system_information", compressed_part);

        Ok(form)
    }

    pub async fn submit_request(
        &self,
        request: Request<ProvingSystemParams>,
    ) -> Result<reqwest::Response> {
        let url = self
            .server_url
            .join("/submit")
            .map_err(|e| RequesterError::ServerUrlParsingError(e.to_string()))?;

        let payload = self.build_multipart(request)?;

        let response = self
            .client
            .post(url)
            .multipart(payload)
            .send()
            .await
            .map_err(|e| RequesterError::ServerRequestError(e.to_string()))?;
        Ok(response)
    }
}
