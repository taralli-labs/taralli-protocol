use super::Risc0Prover;
use crate::error::{Result, WorkerError};
use async_trait::async_trait;
use risc0_zkvm::{default_prover, ExecutorEnv, ProverOpts, Receipt};
use taralli_primitives::systems::risc0::Risc0ProofParams;

pub struct Risc0LocalProver {
    proving_options: ProverOpts,
}

impl Risc0LocalProver {
    #[must_use]
    pub fn new(proving_options: ProverOpts) -> Self {
        Self { proving_options }
    }
}

#[async_trait]
impl Risc0Prover for Risc0LocalProver {
    async fn generate_proof(&self, params: &Risc0ProofParams) -> Result<Receipt> {
        // setup env
        let env = ExecutorEnv::builder()
            .write_slice(&params.inputs)
            .build()
            .map_err(|e| WorkerError::ExecutionFailed(e.to_string()))?;
        let prover = default_prover();
        // generate the proof
        let proof_info = prover
            .prove_with_opts(env, &params.elf, &self.proving_options)
            .map_err(|e| WorkerError::ExecutionFailed(e.to_string()))?;
        Ok(proof_info.receipt)
    }
}
