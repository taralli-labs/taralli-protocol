use taralli_primitives::PrimitivesError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RequesterError {
    #[error("Requester client builder error: {0}")]
    BuilderError(String),
    #[error("Failed to submit request: {0}")]
    RequestSubmissionFailed(String),
    #[error("Error when tracking request: {0}")]
    TrackRequestError(String),
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
    #[error("Failed request signing: {0}")]
    RequestSigningError(String),
    #[error("Failed to parse server url: {0}")]
    ServerUrlParsingError(String),
    #[error("Failed to get permit2 nonce: {0}")]
    GetNonceError(String),
    #[error("Failed to find unused permit2 nonce for configured account")]
    FindUnusedNonceError(),
    #[error("Failed to set timestamps for request, auction length is 0")]
    SetAuctionTimestampsError(),
    #[error("Auction timed out with no Bids")]
    AuctionTimeoutError(),
    #[error("Validation error: {0}")]
    ValidationError(String),
    #[error("Primitives error: {0}")]
    PrimitivesError(#[from] PrimitivesError),
    #[error("API key error: {0}")]
    ApiKeyError(String),
}

pub type Result<T> = core::result::Result<T, RequesterError>;
