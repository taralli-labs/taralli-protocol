use crate::error::{ProviderError, Result};
use crate::worker::{ComputeWorker, WorkResult};
use async_trait::async_trait;
use risc0_zkvm::{default_prover, ExecutorEnv, ProveInfo, ProverOpts, Receipt};
use taralli_primitives::alloy::dyn_abi::dyn_abi::DynSolValue;
use taralli_primitives::alloy::primitives::{Bytes, FixedBytes};
use taralli_primitives::taralli_systems::id::ProvingSystemParams;
use taralli_primitives::taralli_systems::systems::risc0::Risc0ProofParams;
use taralli_primitives::ProofRequest;

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
        log::info!("risc0 worker executor env setup");
        let prover = default_prover();
        // execute risc0 prover
        prover
            .prove_with_opts(env, program, &self.proving_options)
            .map_err(|e| ProviderError::WorkerExecutionFailed(e.to_string()))
    }
}

#[async_trait]
impl ComputeWorker for Risc0Worker {
    async fn execute(&self, request: &ProofRequest<ProvingSystemParams>) -> Result<WorkResult> {
        // prover parameters introspection
        let params = match &request.proving_system_information {
            ProvingSystemParams::Risc0(params) => params.clone(),
            _ => {
                return Err(ProviderError::WorkerExecutionFailed(
                    "Expected Risc0 params".into(),
                ))
            }
        };

        log::info!("risc0 worker: execution started");

        let proof_info = self.generate_proof(&params)?;

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

    use std::path::Path;

    use super::*;
    use alloy::{primitives::U256, sol_types::SolValue};
    use serde_json::Value;

    #[tokio::test]
    async fn test_risc0_worker_execution() -> Result<()> {
        // 1. Load prover inputs
        let elf_path = Path::new("../../contracts/test-proof-data/risc0/is-even");
        let elf = std::fs::read(elf_path)?;

        // 2. Prepare the input (an even number)
        let input_number = U256::from(1304);
        let input_bytes = input_number.abi_encode();

        // 3. Create Risc0ProofParams
        let vm_params = Risc0ProofParams {
            elf,
            inputs: Value::from(input_bytes),
        };

        // 5. Create and execute the worker
        let worker = Risc0Worker::new(ProverOpts::groth16());

        println!("execution starting");
        // 6. Execute and handle the result
        let proof_info = worker.generate_proof(&vm_params).await?;
        println!("execution finished: {:?}", proof_info.receipt);
        Ok(())
    }
}*/
