use axum::{
    async_trait,
    body::Body,
    extract::{FromRequest, Request},
    http::StatusCode,
};
use brotli::Decompressor;
use std::io::{Cursor, Read};

/// A custom extractor that retains both the compressed and decompressed versions.
/// Todo: Remove the decompressed version and only retain the compressed version when/if we stop validation on the server (on submit).
pub struct BrotliFile {
    pub compressed: Vec<u8>,
    pub decompressed: Vec<u8>,
}

#[async_trait]
impl<S> FromRequest<S> for BrotliFile
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request(req: Request<Body>, _state: &S) -> Result<Self, Self::Rejection> {
        let body_bytes = hyper::body::Bytes::from_request(req, _state)
            .await
            .map_err(|_| (StatusCode::BAD_REQUEST, "Error reading request body"))?
            .to_vec();

        let mut decompressor = Decompressor::new(Cursor::new(&body_bytes), 4096);
        let mut decompressed_data = Vec::new();
        decompressor
            .read_to_end(&mut decompressed_data)
            .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid Brotli file"))?;

        Ok(BrotliFile {
            compressed: body_bytes,
            decompressed: decompressed_data,
        })
    }
}
