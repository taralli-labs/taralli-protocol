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
use taralli_primitives::ProofRequest;
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

            log::info!("risc0 bonsai worker executor env setup");
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
    async fn execute(&self, request: &ProofRequest<ProvingSystemParams>) -> Result<WorkResult> {
        // prover parameters introspection
        let params = match &request.proving_system_information {
            ProvingSystemParams::Risc0(params) => params.clone(),
            _ => {
                return Err(ProviderError::WorkerExecutionFailed(
                    "Expected Risc0 Bonsai params".into(),
                ))
            }
        };

        log::info!("risc0 bonsai worker: execution started");
        let proof_info = Self::generate_proof(&params).await.map_err(|e| {
            ProviderError::WorkerExecutionFailed(format!("Failed to generate proof: {}", e))
        })?;

        log::info!("prover execution finished");

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

/*#[cfg(test)]
mod tests {

    use std::path::{Path, PathBuf};

    use super::*;
    use alloy::{primitives::U256, sol_types::SolValue};

    #[tokio::test]
    async fn test_risc0_bonsai_worker_execution() -> Result<()> {
        // Load .env from workspace root
        let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf();

        dotenv::from_path(workspace_root.join(".env"))
            .expect("Failed to load .env file from workspace root");

        // 1. Load prover inputs
        let elf_path = Path::new("../../contracts/test-proof-data/risc0/is-even");
        let elf = std::fs::read(elf_path).expect("elf read failed");

        // 2. Prepare the input (an even number)
        let input_number = U256::from(1304);
        let input_bytes = input_number.abi_encode();

        // 3. Create Risc0ProofParams
        let vm_params: Risc0BonsaiProofParams = Risc0BonsaiProofParams {
            elf: elf.clone(),
            inputs: input_bytes.clone(),
        };

        // 5. Create and execute the worker
        let worker = Risc0BonsaiWorker::default();

        println!("execution starting");
        // 6. Execute and handle the result
        let proof_info = worker.generate_proof(&vm_params).await?;
        println!("execution finished: {:?}", proof_info.stats);
        Ok(())
    }
}*/
