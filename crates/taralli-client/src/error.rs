use taralli_primitives::PrimitivesError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("Config error: {0}")]
    ConfigError(String),
    #[error("Failed to setup bid: {0}")]
    TransactionSetupError(String),
    #[error("Failed to setup event filter: {0}")]
    EventFilterError(String),
    #[error("Failed to parse incoming intent: {0}")]
    IntentParsingError(String),
    #[error("Failed to subscribe to server: {0}")]
    ServerSubscriptionError(String),
    #[error("Failed intent analysis: {0}")]
    IntentAnalysisError(String),
    #[error("Worker failed with error: {0}")]
    WorkerError(String),
    #[error("Client builder error: {0}")]
    BuilderError(String),
    #[error("Failed to submit intent: {0}")]
    IntentSubmissionFailed(String),
    #[error("Failed to decompress intent: {0}")]
    IntentDecompressionFailed(String),
    #[error("Error when tracking intent: {0}")]
    TrackIntentError(String),
    #[error("Failed to send transaction: {0}")]
    TransactionError(String),
    #[error("Transaction failed: {0}")]
    TransactionFailure(String),
    #[error("Failed to parse logs: {0}")]
    LogParseError(String),
    #[error("Failed server request: {0}")]
    ServerRequestError(String),
    #[error("Failed rpc request: {0}")]
    RpcRequestError(String),
    #[error("Failed intent signing: {0}")]
    IntentSigningError(String),
    #[error("Failed to parse server url: {0}")]
    ServerUrlParsingError(String),
    #[error("Failed to get permit2 nonce: {0}")]
    GetNonceError(String),
    #[error("Failed to find unused permit2 nonce for configured account")]
    FindUnusedNonceError(),
    #[error("Failed to set timestamps for intent, auction length is 0")]
    SetAuctionTimestampsError(),
    #[error("Auction timed out with no Bids")]
    AuctionTimeoutError(),
    #[error("Validation error: {0}")]
    ValidationError(String),
    #[error("Primitives error: {0}")]
    PrimitivesError(#[from] PrimitivesError),
    #[error("API key error: {0}")]
    ApiKeyError(String),
    #[error("Invalid client mode: {0}")]
    InvalidMode(String),
    #[error("Provider search is not implemented, error")]
    ProviderSearchingUnimplemented,
}

pub type Result<T> = core::result::Result<T, ClientError>;
