use crate::{
    error::{ProviderError, Result},
    worker::{ComputeWorker, WorkResult},
};
use async_trait::async_trait;
use serde::Serialize;
use serde_json::Value;
use std::path::PathBuf;
use taralli_primitives::alloy::primitives::{Bytes, FixedBytes};
use taralli_primitives::taralli_systems::id::ProvingSystemParams;
use taralli_primitives::taralli_systems::systems::gnark::{GnarkProofParams, GnarkSchemeConfig};
use taralli_primitives::ProofRequest;
use tempfile::NamedTempFile;

#[derive(Default)]
pub struct GnarkWorker;

impl GnarkWorker {
    pub fn new() -> Self {
        Self
    }

    // WIP also this
    fn format_opaque_submission(_proof: Vec<u8>, _public_inputs: Value) -> Result<Bytes> {
        Ok(Bytes::from(FixedBytes::<32>::ZERO))
    }

    fn compute_partial_commitment() -> Result<FixedBytes<32>> {
        Ok(FixedBytes::new([0u8; 32]))
    }

    pub async fn execute_gnark_prover(gnark_params: &GnarkProofParams) -> Result<PathBuf> {
        let mut params_file = NamedTempFile::new()
            .map_err(|e| ProviderError::WorkerExecutionFailed(e.to_string()))?;
        let proof_output_file = NamedTempFile::new()
            .map_err(|e| ProviderError::WorkerExecutionFailed(e.to_string()))?;

        // Create a serializable structure that matches GnarkProofParams
        #[derive(Serialize)]
        struct GnarkProverInput {
            r1cs: Vec<u8>,
            public_inputs: Value,
            private_inputs: Value,
            scheme_config: String,
            curve: String,
        }

        // Build command based on scheme configuration
        let (scheme, curve) = match gnark_params.scheme_config {
            GnarkSchemeConfig::Groth16Bn254 => ("groth16", "bn254"),
            GnarkSchemeConfig::PlonkBn254 => ("plonk", "bn254"),
            GnarkSchemeConfig::PlonkBls12_381 => ("plonk", "bls12-381"),
        };

        // Create the input structure
        let prover_input = GnarkProverInput {
            r1cs: gnark_params.r1cs.clone(),
            public_inputs: gnark_params.public_inputs.clone(),
            private_inputs: gnark_params.input.clone(),
            scheme_config: scheme.to_string(),
            curve: curve.to_string(),
        };

        // Write all params to a single JSON file
        serde_json::to_writer(&mut params_file, &prover_input)
            .map_err(|e| ProviderError::WorkerExecutionFailed(e.to_string()))?;

        // Execute the Go prover with single input file
        let prover_output = tokio::process::Command::new("gnark-prover")
            .arg("--params")
            .arg(params_file.path())
            .arg("--output")
            .arg(proof_output_file.path())
            .output()
            .await
            .map_err(|e| ProviderError::WorkerExecutionFailed(e.to_string()))?;

        if !prover_output.status.success() {
            return Err(ProviderError::WorkerExecutionFailed(
                String::from_utf8_lossy(&prover_output.stderr).to_string(),
            ));
        }

        Ok(proof_output_file.path().to_path_buf())
    }

    async fn generate_proof(&self, gnark_params: &GnarkProofParams) -> Result<(Vec<u8>, Value)> {
        // run gnark prover
        let proof_output_path = Self::execute_gnark_prover(&gnark_params).await?;

        // Read proof from output file
        let proof = std::fs::read(proof_output_path)
            .map_err(|e| ProviderError::WorkerExecutionFailed(e.to_string()))?;

        Ok((proof, gnark_params.public_inputs.clone()))
    }
}

#[async_trait]
impl ComputeWorker for GnarkWorker {
    async fn execute(&self, request: &ProofRequest<ProvingSystemParams>) -> Result<WorkResult> {
        log::info!("gnark worker: execution started");

        let params = match &request.proving_system_information {
            ProvingSystemParams::Gnark(params) => params.clone(),
            _ => {
                return Err(ProviderError::WorkerExecutionFailed(
                    "Expected Gnark params".into(),
                ))
            }
        };

        // Generate proof
        let (proof, public_inputs) = self.generate_proof(&params).await?;

        // Format proof data for resolution
        let opaque_submission = Self::format_opaque_submission(proof, public_inputs)?;
        // get empty partial commitment (no op)
        let partial_commitment = Self::compute_partial_commitment()?;
        Ok(WorkResult {
            opaque_submission,
            partial_commitment,
        })
    }
}

/*#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use color_eyre::Result;

    #[tokio::test]
    async fn test_direct_proof_generation() -> Result<()> {

    }
}*/
