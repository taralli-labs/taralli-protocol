use taralli_primitives::PrimitivesError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ServerError {
    #[error("App state error: -> {0}")]
    AppStateError(String),
    #[error("Validation: Error when fetching latest block")]
    FetchLatestBlockTimestampError,
    #[error("Submit: validation timed out after {0} seconds")]
    ValidationTimeout(u64),
    #[error("Submit: validation error -> {0}")]
    ValidationError(String),
    #[error("Subscription manager: no proof providers available for selected proving system.")]
    NoProvidersAvailable(),
    #[error("Broadcast failed: {0}")]
    BroadcastError(String),
    #[error("Primitives error: {0}")]
    PrimitivesError(#[from] PrimitivesError),
}

pub type Result<T> = core::result::Result<T, ServerError>;
