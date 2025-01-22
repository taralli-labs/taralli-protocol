pub mod local; // local risc0 prover
pub mod remote; // bonsai network risc0 prover

use async_trait::async_trait;
use risc0_zkvm::Receipt;
use taralli_primitives::systems::risc0::Risc0ProofParams;

use crate::error::{ProviderError, Result};
use crate::worker::{ComputeWorker, WorkResult};
use taralli_primitives::alloy::dyn_abi::dyn_abi::DynSolValue;
use taralli_primitives::alloy::primitives::{Bytes, FixedBytes};
use taralli_primitives::systems::ProvingSystemParams;
use taralli_primitives::Request;

// Shared traits & functionality for all RISC0 workers
pub trait Risc0ProofFormatter {
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
}

#[async_trait]
pub trait Risc0Prover {
    async fn generate_proof(&self, params: &Risc0ProofParams) -> Result<Receipt>;
}

pub struct Risc0Worker<P: Risc0Prover> {
    prover: P,
}

impl<P: Risc0Prover> Risc0Worker<P> {
    pub fn new(prover: P) -> Self {
        Self { prover }
    }
}

impl<P: Risc0Prover> Risc0ProofFormatter for Risc0Worker<P> {}

#[async_trait]
impl<P: Risc0Prover + Send + Sync> ComputeWorker for Risc0Worker<P> {
    async fn execute(&self, request: &Request<ProvingSystemParams>) -> Result<WorkResult> {
        let params = match &request.proving_system_information {
            ProvingSystemParams::Risc0(params) => params.clone(),
            _ => {
                return Err(ProviderError::WorkerExecutionFailed(
                    "Expected Risc0 params".into(),
                ))
            }
        };

        tracing::info!("risc0 worker: execution started");
        let receipt = self.prover.generate_proof(&params).await?;
        tracing::info!("prover execution finished");

        let image_id = FixedBytes::from_slice(&params.elf[0..32]);
        let opaque_submission = Self::format_opaque_submission(&receipt, image_id)?;
        let partial_commitment = Self::compute_partial_commitment(&receipt.journal.bytes)?;

        Ok(WorkResult {
            opaque_submission,
            partial_commitment,
        })
    }
}
