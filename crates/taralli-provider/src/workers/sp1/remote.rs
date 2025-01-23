use std::time::Duration;

use super::Sp1Prover;
use crate::error::{ProviderError, Result};
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
            .map_err(|e| ProviderError::WorkerExecutionFailed(e.to_string()))?;

        // Wait for proof complete with a timeout
        let _proof = self
            .network_prover
            .wait_proof(request_id, Some(Duration::from_secs(60 * 60)))
            .await
            .map_err(|e| ProviderError::WorkerExecutionFailed(e.to_string()))?;

        // Request a proof with reserved prover network capacity and wait for it to be fulfilled
        let proof = self
            .network_prover
            .prove(&pk, &stdin)
            .mode(self.proof_mode)
            .skip_simulation(self.simulate_locally)
            .strategy(self.fulfillment_strategy)
            .run_async()
            .await
            .map_err(|e| ProviderError::WorkerExecutionFailed(e.to_string()))?;

        Ok((proof, vk))
    }
}

/*#[cfg(test)]
mod tests {
    use std::path::Path;
    use super::*;

    #[tokio::test]
    async fn test_succint_prover_network() {
        // Load .env from workspace root
        dotenv::from_filename("../../.env").expect("Failed to load .env file");
        // Read API key and URL from environment, or skip test if not available
        let priv_key = std::env::var("SUCCINCT_PRIVATE_KEY").expect("SUCCINCT_PRIVATE_KEY not set after loading .env");
        let api_url = std::env::var("SUCCINT_RPC_URL").expect("SUCCINT_RPC_URL not set after loading .env");

        // proving system information data
        let sp1_program_path = Path::new("./contracts/test-proof-data/sp1/fibonacci-program");
        let elf = std::fs::read(sp1_program_path).expect("reading elf failed");
        // proof input(s)
        let inputs = 1000u32;
        let input_bytes = inputs.to_le_bytes().to_vec();

        let mut stdin = SP1Stdin::new();
        stdin.write(&input_bytes);

        // set up succint network prover
        let prover = ProverClient::builder()
            .network()
            .private_key(&priv_key)
            .rpc_url(&api_url)
            .build();

        let (pk, _vk) = prover.setup(&elf);

        // Request proof and get the proof ID immediately
        let request_id = prover.prove(&pk, &stdin).groth16().skip_simulation(true).request_async().await.expect("sp1 proving failed");
        println!("Proof request ID: {}", request_id);

        // Wait for proof complete with a timeout
        let proof = prover.wait_proof(request_id, Some(Duration::from_secs(60 * 60))).await.expect("wait proof failed");

        println!("proof: {:?}", proof);
    }
}*/
