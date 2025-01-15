use crate::error::{ProviderError, Result};
use crate::worker::{ComputeWorker, WorkResult};
use async_trait::async_trait;
use risc0_zkvm::{
    BonsaiProver, ExecutorEnv, ProveInfo, Prover, ProverOpts, Receipt, VerifierContext,
};
use taralli_primitives::alloy::dyn_abi::dyn_abi::DynSolValue;
use taralli_primitives::alloy::primitives::{Bytes, FixedBytes};
use taralli_primitives::taralli_systems::id::ProvingSystemParams;
use taralli_primitives::taralli_systems::systems::risc0::Risc0ProofParams;
use taralli_primitives::Request;
use tokio::task;

#[derive(Default)]
pub struct Risc0BonsaiWorker;

impl Risc0BonsaiWorker {
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

    async fn generate_proof(params: &Risc0ProofParams) -> Result<ProveInfo> {
        let program = params.elf.clone();
        let inputs = params.inputs.clone();

        // Spawn a blocking task to handle the Bonsai operations
        let proof_info = task::spawn_blocking(move || {
            // setup prover env
            let env = ExecutorEnv::builder()
                .write_slice(&inputs)
                .build()
                .map_err(|e| ProviderError::WorkerExecutionFailed(e.to_string()))?;

            tracing::info!("risc0 bonsai worker executor env setup");
            let prover = BonsaiProver::new("bonsai prover");

            // execute risc0 prover
            prover
                .prove_with_ctx(
                    env,
                    &VerifierContext::default(),
                    &program,
                    &ProverOpts::groth16(),
                )
                .map_err(|e| ProviderError::WorkerExecutionFailed(e.to_string()))
        })
        .await
        .map_err(|e| ProviderError::WorkerExecutionFailed(e.to_string()))??;

        Ok(proof_info)
    }
}

#[async_trait]
impl ComputeWorker for Risc0BonsaiWorker {
    async fn execute(&self, request: &Request<ProvingSystemParams>) -> Result<WorkResult> {
        // prover parameters introspection
        let params = match &request.proving_system_information {
            ProvingSystemParams::Risc0(params) => params.clone(),
            _ => {
                return Err(ProviderError::WorkerExecutionFailed(
                    "Expected Risc0 Bonsai params".into(),
                ))
            }
        };

        tracing::info!("risc0 bonsai worker: execution started");
        let proof_info = Self::generate_proof(&params).await.map_err(|e| {
            ProviderError::WorkerExecutionFailed(format!("Failed to generate proof: {}", e))
        })?;

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
