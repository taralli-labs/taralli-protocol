use thiserror::Error;

#[derive(Error, Debug)]
pub enum PrimitivesError {
    #[error("Contract interaction error: {0}")]
    ContractError(String),
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
}

pub type Result<T> = core::result::Result<T, PrimitivesError>;
