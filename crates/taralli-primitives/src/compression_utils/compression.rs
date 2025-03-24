use std::io::Write;

use async_compression::tokio::bufread::BrotliDecoder;
use tokio::io::AsyncReadExt;

use crate::{
    error::{PrimitivesError, Result},
    systems::SystemParams,
};

/// Compresses the bytes payload using Brotli compression
/// and returns the compressed payload as a byte vector
/// # Arguments
/// * `intent` - The intent to be compressed
/// # Returns
/// * A byte vector containing the compressed payload
/// # Details
/// The compression level, buffer size, and window size are configurable
/// via the environment variables.
/// Furthermore, we chose to instantiate a new compressor for each intent
/// if the need to submit multiple intent concurrently arises.
pub fn compress_brotli(payload: String) -> Result<Vec<u8>> {
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
    brotli_encoder
        .write_all(payload.as_bytes())
        .map_err(|e| PrimitivesError::CompressionError(e.to_string()))?;
    Ok(brotli_encoder.into_inner())
}

/// Decompress a Brotli-compressed byte vector
/// # Arguments
/// * `compressed_bytes` - The Brotli-compressed byte vector
/// # Returns
/// * A byte vector containing the decompressed data
pub async fn decompress_system(compressed_bytes: Vec<u8>) -> Result<SystemParams> {
    let decompressed = decompress_brotli(compressed_bytes).await?;
    let params = serde_json::from_slice(&decompressed)
        .map_err(|e| PrimitivesError::DecompressionError(e.to_string()))?;
    Ok(params)
}

/// Decompress a Brotli-compressed byte vector
/// # Arguments
/// * `compressed_bytes` - The Brotli-compressed byte vector
/// # Returns
/// * A byte vector containing the decompressed data
pub async fn decompress_brotli(compressed_bytes: Vec<u8>) -> Result<Vec<u8>> {
    let mut decoder = BrotliDecoder::new(tokio::io::BufReader::new(&compressed_bytes[..]));
    let mut decompressed = Vec::new();
    decoder
        .read_to_end(&mut decompressed)
        .await
        .map_err(|e| PrimitivesError::DecompressionError(e.to_string()))?;
    Ok(decompressed)
}
