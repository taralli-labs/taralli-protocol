use std::time::Duration;

use super::Sp1Prover;
use crate::error::{Result, WorkerError};
use async_trait::async_trait;
use sp1_sdk::{
    network::FulfillmentStrategy, NetworkProver, Prover, ProverClient, SP1ProofMode,
    SP1ProofWithPublicValues, SP1Stdin, SP1VerifyingKey,
};
use taralli_primitives::systems::sp1::Sp1ProofParams;

pub struct Sp1RemoteProver {
    network_prover: NetworkProver,
    fulfillment_strategy: FulfillmentStrategy,
    proof_mode: SP1ProofMode,
    simulate_locally: bool,
}

impl Sp1RemoteProver {
    pub fn new(
        private_key: &str,
        rpc_url: &str,
        fulfillment_strategy: FulfillmentStrategy,
        proof_mode: SP1ProofMode,
        simulate_locally: bool,
    ) -> Self {
        // set up succint network prover
        let network_prover = ProverClient::builder()
            .network()
            .private_key(private_key)
            .rpc_url(rpc_url)
            .build();

        Self {
            network_prover,
            fulfillment_strategy,
            proof_mode,
            simulate_locally,
        }
    }
}

#[async_trait]
impl Sp1Prover for Sp1RemoteProver {
    async fn generate_proof(
        &self,
        params: &Sp1ProofParams,
    ) -> Result<(SP1ProofWithPublicValues, SP1VerifyingKey)> {
        let mut stdin = SP1Stdin::new();
        stdin.write(&params.inputs);

        let (pk, vk) = self.network_prover.setup(&params.elf);

        // Request proof and get the proof ID immediately
        let request_id = self
            .network_prover
            .prove(&pk, &stdin)
            .groth16()
            .skip_simulation(true)
            .request_async()
            .await
            .map_err(|e| WorkerError::ExecutionFailed(e.to_string()))?;

        // Wait for proof complete with a timeout
        let _proof = self
            .network_prover
            .wait_proof(request_id, Some(Duration::from_secs(60 * 60)))
            .await
            .map_err(|e| WorkerError::ExecutionFailed(e.to_string()))?;

        // Request a proof with reserved prover network capacity and wait for it to be fulfilled
        let proof = self
            .network_prover
            .prove(&pk, &stdin)
            .mode(self.proof_mode)
            .skip_simulation(self.simulate_locally)
            .strategy(self.fulfillment_strategy)
            .run_async()
            .await
            .map_err(|e| WorkerError::ExecutionFailed(e.to_string()))?;

        Ok((proof, vk))
    }
}
