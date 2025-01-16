use crate::error::{ProviderError, Result};
use crate::worker::{ComputeWorker, WorkResult};
use async_trait::async_trait;
use risc0_zkvm::{default_prover, ExecutorEnv, ProveInfo, ProverOpts, Receipt};
use taralli_primitives::alloy::dyn_abi::dyn_abi::DynSolValue;
use taralli_primitives::alloy::primitives::{Bytes, FixedBytes};
use taralli_primitives::taralli_systems::id::ProvingSystemParams;
use taralli_primitives::taralli_systems::systems::risc0::Risc0ProofParams;
use taralli_primitives::Request;

pub struct Risc0Worker {
    proving_options: ProverOpts,
}

impl Risc0Worker {
    pub fn new(proving_options: ProverOpts) -> Self {
        Self { proving_options }
    }

    fn format_opaque_submission(receipt: &Receipt, image_id: FixedBytes<32>) -> Result<Bytes> {
        let proof_input_values = DynSolValue::Tuple(vec![
            DynSolValue::Bytes(
                receipt
                    .inner
                    .groth16()
                    .map_err(|e| ProviderError::WorkerExecutionFailed(e.to_string()))?
                    .seal
                    .clone(),
            ),
            DynSolValue::FixedBytes(image_id, 32),
            DynSolValue::FixedBytes(FixedBytes::from_slice(&receipt.journal.bytes), 32),
        ]);

        Ok(Bytes::from(proof_input_values.abi_encode()))
    }

    fn compute_partial_commitment(_journal: &[u8]) -> Result<FixedBytes<32>> {
        Ok(FixedBytes::new([0u8; 32]))
    }

    // NOTE only works on x86 ISA hardware, configured through env var to allow building even on apple silicon
    pub fn generate_proof(&self, params: &Risc0ProofParams) -> Result<ProveInfo> {
        let program = &params.elf;
        let env = ExecutorEnv::builder()
            .write_slice(&params.inputs)
            .build()
            .map_err(|e| ProviderError::WorkerExecutionFailed(e.to_string()))?;
        tracing::info!("risc0 worker executor env setup");
        let prover = default_prover();
        // execute risc0 prover
        prover
            .prove_with_opts(env, program, &self.proving_options)
            .map_err(|e| ProviderError::WorkerExecutionFailed(e.to_string()))
    }
}

#[async_trait]
impl ComputeWorker for Risc0Worker {
    async fn execute(&self, request: &Request<ProvingSystemParams>) -> Result<WorkResult> {
        // prover parameters introspection
        let params = match &request.proving_system_information {
            ProvingSystemParams::Risc0(params) => params.clone(),
            _ => {
                return Err(ProviderError::WorkerExecutionFailed(
                    "Expected Risc0 params".into(),
                ))
            }
        };

        tracing::info!("risc0 worker: execution started");

        let proof_info = self.generate_proof(&params)?;

        tracing::info!("prover execution finished");

        let image_id = FixedBytes::from_slice(&params.elf[0..32]);
        let opaque_submission = Self::format_opaque_submission(&proof_info.receipt, image_id)?;
        let partial_commitment =
            Self::compute_partial_commitment(&proof_info.receipt.journal.bytes)?;
        Ok(WorkResult {
            opaque_submission,
            partial_commitment,
        })
    }
}
