use std::str::FromStr;

use crate::error::{ProviderError, Result};
use crate::worker::{ComputeWorker, WorkResult};
use async_trait::async_trait;
use sp1_sdk::{HashableKey, ProverClient, SP1ProofWithPublicValues, SP1Stdin, SP1VerifyingKey};
use taralli_primitives::alloy::dyn_abi::dyn_abi::DynSolValue;
use taralli_primitives::alloy::primitives::{Bytes, FixedBytes};
use taralli_primitives::systems::sp1::Sp1ProofParams;
use taralli_primitives::systems::ProvingSystemParams;
use taralli_primitives::Request;

pub struct Sp1Worker {
    prover_client: ProverClient,
}

impl Sp1Worker {
    pub fn new(prover_client: ProverClient) -> Self {
        Self { prover_client }
    }

    fn format_opaque_submission(
        sp1_proof: &SP1ProofWithPublicValues,
        vk: &SP1VerifyingKey,
    ) -> Result<Bytes> {
        // check that proof type is either groth16 or plonk as these are the only on chain veriable
        // proof types for sp1 proofs
        let proof_bytes = match sp1_proof.proof {
            sp1_sdk::SP1Proof::Plonk(_) | sp1_sdk::SP1Proof::Groth16(_) => Ok(sp1_proof.bytes()),
            _ => Err(ProviderError::WorkerExecutionFailed(
                "SP1 proof must be of type groth16 or plonk to be verified on-chain".to_string(),
            )),
        }?;

        let vkey = FixedBytes::from_str(&vk.bytes32())
            .map_err(|e| ProviderError::WorkerExecutionFailed(e.to_string()))?;

        let proof_input_values = DynSolValue::Tuple(vec![
            DynSolValue::FixedBytes(vkey, 32),
            DynSolValue::Bytes(sp1_proof.public_values.to_vec()),
            DynSolValue::Bytes(proof_bytes),
        ]);

        Ok(Bytes::from(proof_input_values.abi_encode()))
    }

    fn compute_partial_commitment() -> Result<FixedBytes<32>> {
        Ok(FixedBytes::new([0u8; 32]))
    }

    pub fn generate_proof(
        &self,
        params: &Sp1ProofParams,
    ) -> Result<(SP1ProofWithPublicValues, SP1VerifyingKey)> {
        // write inputs
        let mut stdin = SP1Stdin::new();
        stdin.write(&params.inputs);
        // setup
        let (pk, vk) = self.prover_client.setup(params.elf.as_slice());
        // Generate the proof for the given program and input.
        let sp1_proof = self
            .prover_client
            .prove(&pk, stdin)
            .run()
            .map_err(|e| ProviderError::WorkerExecutionFailed(e.to_string()))?;
        Ok((sp1_proof, vk))
    }
}

#[async_trait]
impl ComputeWorker for Sp1Worker {
    async fn execute(&self, request: &Request<ProvingSystemParams>) -> Result<WorkResult> {
        // prover parameters introspection
        let params = match &request.proving_system_information {
            ProvingSystemParams::Sp1(params) => params.clone(),
            _ => {
                return Err(ProviderError::WorkerExecutionFailed(
                    "Expected Sp1 params".into(),
                ))
            }
        };

        tracing::info!("Sp1 worker: execution started");
        let (sp1_proof, vk) = self.generate_proof(&params).map_err(|e| {
            ProviderError::WorkerExecutionFailed(format!("Failed to generate proof: {}", e))
        })?;

        tracing::info!("prover execution finished");
        let opaque_submission = Self::format_opaque_submission(&sp1_proof, &vk)?;
        let partial_commitment = Self::compute_partial_commitment()?;

        Ok(WorkResult {
            opaque_submission,
            partial_commitment,
        })
    }
}
