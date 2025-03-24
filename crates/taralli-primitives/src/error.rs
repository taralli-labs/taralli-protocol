use thiserror::Error;

#[derive(Error, Debug)]
pub enum PrimitivesError {
    #[error("Compression error: {0}")]
    CompressionError(String),
    #[error("Decompression error: {0}")]
    DecompressionError(String),
    #[error("Contract interaction error: {0}")]
    ContractError(String),
    #[error("Configuration error: {0}")]
    ConfigError(String),
    #[error("Invalid signature: {0}")]
    SignatureError(String),
    #[error("Validation error: {0}")]
    ValidationError(String),
    #[error("RPC error: {0}")]
    RpcError(String),
    #[error("Encoding error: {0}")]
    EncodingError(String),
    #[error("Commitment error: {0}")]
    CommitmentError(String),
    #[error("Prover Inputs validation error: {0}")]
    ProverInputsError(String),
    #[error("Invalid systems error: {0}")]
    InvalidSystem(String),
    #[error("Intent serialization error: {0}")]
    SerializationError(String),
    #[error("DB serialization error: {0}")]
    DbSerializeError(String),
    #[error("DB serialization error: {0}")]
    DbDeserializeError(String),
}

pub type Result<T> = core::result::Result<T, PrimitivesError>;
