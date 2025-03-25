use super::Sp1Prover;
use crate::error::{Result, WorkerError};
use async_trait::async_trait;
use sp1_sdk::{
    CpuProver, CudaProver, Prover, ProverClient, SP1ProofMode, SP1ProofWithPublicValues, SP1Stdin,
    SP1VerifyingKey,
};
use taralli_primitives::systems::sp1::Sp1ProofParams;

pub enum Sp1LocalProverType {
    Cpu(CpuProver),
    Cuda(CudaProver),
}

pub struct Sp1LocalProver {
    prover: Sp1LocalProverType,
    proof_mode: SP1ProofMode,
}

impl Sp1LocalProver {
    #[must_use]
    pub fn new(use_cuda: bool, proof_mode: SP1ProofMode) -> Self {
        let prover = if use_cuda {
            Sp1LocalProverType::Cuda(ProverClient::builder().cuda().build())
        } else {
            Sp1LocalProverType::Cpu(ProverClient::builder().cpu().build())
        };

        Self { prover, proof_mode }
    }
}

#[async_trait]
impl Sp1Prover for Sp1LocalProver {
    async fn generate_proof(
        &self,
        params: &Sp1ProofParams,
    ) -> Result<(SP1ProofWithPublicValues, SP1VerifyingKey)> {
        let mut stdin = SP1Stdin::new();
        stdin.write(&params.inputs);

        match &self.prover {
            Sp1LocalProverType::Cpu(prover) => {
                let (pk, vk) = prover.setup(&params.elf);
                let proof = prover
                    .prove(&pk, &stdin)
                    .mode(self.proof_mode)
                    .run()
                    .map_err(|e| WorkerError::ExecutionFailed(e.to_string()))?;
                Ok((proof, vk))
            }
            Sp1LocalProverType::Cuda(prover) => {
                let (pk, vk) = prover.setup(&params.elf);
                let proof = prover
                    .prove(&pk, &stdin)
                    .mode(self.proof_mode)
                    .run()
                    .map_err(|e| WorkerError::ExecutionFailed(e.to_string()))?;
                Ok((proof, vk))
            }
        }
    }
}
