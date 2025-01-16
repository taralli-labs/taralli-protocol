//! Core types and traits for the Taralli protocol
//!
//! This module re-exports commonly used types from alloy-primitives
//! to ensure version compatibility and provide a single source of truth.

pub mod alloy {
    pub mod primitives {
        pub use alloy::primitives::{
            address, b256, bytes, fixed_bytes, Address, Bytes, FixedBytes, U256,
        };
    }
}

use crate::error::Result;
use alloy::primitives::{Address, FixedBytes, U256};
use serde::Serialize;

pub mod error;
pub mod id;
pub mod systems;

#[derive(Debug, Default)]
pub struct VerifierConstraints {
    pub verifier: Option<Address>,
    pub selector: Option<FixedBytes<4>>,
    pub is_sha_commitment: Option<bool>,
    pub public_inputs_offset: Option<U256>,
    pub public_inputs_length: Option<U256>,
    pub has_partial_commitment_result_check: Option<bool>,
    pub submitted_partial_commitment_result_offset: Option<U256>,
    pub submitted_partial_commitment_result_length: Option<U256>,
    pub predetermined_partial_commitment: Option<FixedBytes<32>>,
}

pub trait ProvingSystemInformation: Send + Sync + Clone + Serialize + 'static {
    fn validate_prover_inputs(&self) -> Result<()>;
    fn verifier_constraints() -> VerifierConstraints;
}
