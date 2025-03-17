pub mod local; // local sp1 prover
pub mod remote; // succint network sp1 prover

use crate::error::{Result, WorkerError};
use alloy::{
    dyn_abi::DynSolValue,
    primitives::{Bytes, FixedBytes},
};
use async_trait::async_trait;
use sp1_sdk::{HashableKey, SP1ProofWithPublicValues, SP1VerifyingKey};
use std::str::FromStr;
use taralli_client::{
    error::ClientError,
    worker::{ComputeWorker, WorkResult},
};
use taralli_primitives::{
    intents::ComputeIntent,
    systems::{sp1::Sp1ProofParams, System, SystemParams},
};

pub trait Sp1ProofFormatter {
    fn format_opaque_submission(
        sp1_proof: &SP1ProofWithPublicValues,
        vk: &SP1VerifyingKey,
    ) -> Result<Bytes> {
        // check that proof type is either groth16 or plonk as these are the only on chain veriable
        // proof types for sp1 proofs
        let proof_bytes = match sp1_proof.proof {
            sp1_sdk::SP1Proof::Plonk(_) | sp1_sdk::SP1Proof::Groth16(_) => Ok(sp1_proof.bytes()),
            _ => Err(WorkerError::ExecutionFailed(
                "SP1 proof must be of type groth16 or plonk to be verified on-chain".to_string(),
            )),
        }?;

        let vkey = FixedBytes::from_str(&vk.bytes32())
            .map_err(|e| WorkerError::ExecutionFailed(e.to_string()))?;

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
}

#[async_trait]
pub trait Sp1Prover {
    async fn generate_proof(
        &self,
        params: &Sp1ProofParams,
    ) -> Result<(SP1ProofWithPublicValues, SP1VerifyingKey)>;
}

pub struct Sp1Worker<P: Sp1Prover> {
    prover: P,
}

impl<P: Sp1Prover> Sp1Worker<P> {
    pub fn new(prover: P) -> Self {
        Self { prover }
    }
}

impl<P: Sp1Prover> Sp1ProofFormatter for Sp1Worker<P> {}

#[async_trait]
impl<I, P> ComputeWorker<I> for Sp1Worker<P>
where
    P: Sp1Prover + Send + Sync,
    I: ComputeIntent<System = SystemParams> + Send + Sync,
{
    async fn execute(&self, intent: &I) -> taralli_client::error::Result<WorkResult> {
        tracing::info!("Sp1 worker: execution started");

        let system_params = intent
            .system()
            .system_params()
            .ok_or_else(|| ClientError::WorkerError("System params not available".into()))?;

        // prover parameters introspection
        let params = match system_params {
            SystemParams::Sp1(params) => params.clone(),
            _ => return Err(ClientError::WorkerError("Expected Sp1 params".into())),
        };

        let (sp1_proof, vk) = self.prover.generate_proof(&params).await?;
        tracing::info!("prover execution finished");

        let opaque_submission = Self::format_opaque_submission(&sp1_proof, &vk)?;
        let partial_commitment = Self::compute_partial_commitment()?;

        Ok(WorkResult {
            opaque_submission,
            partial_commitment,
        })
    }
}
