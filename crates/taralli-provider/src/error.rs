use taralli_primitives::PrimitivesError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("Failed to setup bid: {0}")]
    TransactionSetupError(String),
    #[error("Provider client builder error: {0}")]
    BuilderError(String),
    #[error("Failed to setup event filter: {0}")]
    EventFilterError(String),
    #[error("Failed to send transaction: {0}")]
    TransactionError(String),
    #[error("Transaction failed: {0}")]
    TransactionFailure(String),
    #[error("Failed to parse logs: {0}")]
    LogParseError(String),
    #[error("Failed server request: {0}")]
    ServerRequestError(String),
    #[error("Failed to parse incoming request: {0}")]
    RequestParsingError(String),
    #[error("Failed to subscribe to server: {0}")]
    ServerSubscriptionError(String),
    #[error("Failed rpc request: {0}")]
    RpcRequestError(String),
    #[error("Failed request analysis: {0}")]
    RequestAnalysisError(String),
    #[error("Failed to execute worker: {0}")]
    WorkerExecutionFailed(String),
    #[error("Primitives error: {0}")]
    PrimitivesError(#[from] PrimitivesError),
}

pub type Result<T> = core::result::Result<T, ProviderError>;
