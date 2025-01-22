use thiserror::Error;

#[derive(Error, Debug)]
pub enum SystemsError {
    #[error("Prover Inputs validation error: {0}")]
    ProverInputsError(String),
}

pub type Result<T> = core::result::Result<T, SystemsError>;
