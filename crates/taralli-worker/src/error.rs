use taralli_client::error::ClientError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum WorkerError {
    #[error("Failed to setup worker params: {0}")]
    ParamsError(String),
    #[error("Failed to execute worker: {0}")]
    ExecutionFailed(String),
}

// Implement conversion from WorkerError to ClientError
impl From<WorkerError> for ClientError {
    fn from(err: WorkerError) -> Self {
        match err {
            WorkerError::ExecutionFailed(msg) => ClientError::WorkerError(msg),
            WorkerError::ParamsError(msg) => ClientError::WorkerError(msg),
        }
    }
}

pub type Result<T> = core::result::Result<T, WorkerError>;
