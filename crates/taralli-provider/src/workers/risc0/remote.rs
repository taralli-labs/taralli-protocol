use std::time::Duration;

use super::Risc0Prover;
use crate::error::{ProviderError, Result};
use async_trait::async_trait;
use bonsai_sdk::non_blocking::Client;
use risc0_zkvm::Receipt;
use risc0_zkvm::{compute_image_id, serde::to_vec};
use taralli_primitives::systems::risc0::Risc0ProofParams;

pub struct Risc0RemoteProver;

#[async_trait]
impl Risc0Prover for Risc0RemoteProver {
    async fn generate_proof(&self, params: &Risc0ProofParams) -> Result<Receipt> {
        let program = params.elf.clone();
        let inputs = params.inputs.clone();

        // Create async client
        let client = Client::from_env(risc0_zkvm::VERSION)
            .map_err(|e| ProviderError::WorkerExecutionFailed(e.to_string()))?;

        // Compute and upload image ID
        let image_id = hex::encode(
            compute_image_id(&program)
                .map_err(|e| ProviderError::WorkerExecutionFailed(e.to_string()))?,
        );

        client
            .upload_img(&image_id, program)
            .await
            .map_err(|e| ProviderError::WorkerExecutionFailed(e.to_string()))?;

        // Prepare and upload input data
        let input_data =
            to_vec(&inputs).map_err(|e| ProviderError::WorkerExecutionFailed(e.to_string()))?;
        let input_data = bytemuck::cast_slice(&input_data).to_vec();
        let input_id = client
            .upload_input(input_data)
            .await
            .map_err(|e| ProviderError::WorkerExecutionFailed(e.to_string()))?;

        // Create session
        let session = client
            .create_session(image_id.clone(), input_id, vec![], false)
            .await
            .map_err(|e| ProviderError::WorkerExecutionFailed(e.to_string()))?;

        // Poll for STARK proof completion
        let _receipt_url = loop {
            let res = session
                .status(&client)
                .await
                .map_err(|e| ProviderError::WorkerExecutionFailed(e.to_string()))?;

            match res.status.as_str() {
                "RUNNING" => {
                    tracing::info!(
                        "Bonsai STARK proof status: {} - state: {}",
                        res.status,
                        res.state.unwrap_or_default()
                    );
                    tokio::time::sleep(Duration::from_secs(15)).await;
                    continue;
                }
                "SUCCEEDED" => {
                    break res.receipt_url.ok_or_else(|| {
                        ProviderError::WorkerExecutionFailed(
                            "API error: missing receipt on completed session".into(),
                        )
                    })?;
                }
                _ => {
                    return Err(ProviderError::WorkerExecutionFailed(format!(
                        "Bonsai workflow failed: {} - error: {}",
                        res.status,
                        res.error_msg.unwrap_or_default()
                    )));
                }
            }
        };

        // Create SNARK proof
        let snark_session = client
            .create_snark(session.uuid)
            .await
            .map_err(|e| ProviderError::WorkerExecutionFailed(e.to_string()))?;

        // Poll for SNARK proof completion
        let snark_receipt = loop {
            let res = snark_session
                .status(&client)
                .await
                .map_err(|e| ProviderError::WorkerExecutionFailed(e.to_string()))?;

            match res.status.as_str() {
                "RUNNING" => {
                    tracing::info!("Bonsai SNARK proof status: {}", res.status);
                    tokio::time::sleep(Duration::from_secs(15)).await;
                    continue;
                }
                "SUCCEEDED" => {
                    let receipt_buf = client
                        .download(&res.output.ok_or_else(|| {
                            ProviderError::WorkerExecutionFailed("Missing SNARK output URL".into())
                        })?)
                        .await
                        .map_err(|e| ProviderError::WorkerExecutionFailed(e.to_string()))?;

                    break bincode::deserialize(&receipt_buf)
                        .map_err(|e| ProviderError::WorkerExecutionFailed(e.to_string()))?;
                }
                _ => {
                    return Err(ProviderError::WorkerExecutionFailed(format!(
                        "SNARK workflow failed: {} - error: {}",
                        res.status,
                        res.error_msg.unwrap_or_default()
                    )));
                }
            }
        };

        Ok(snark_receipt)
    }
}

/*#[cfg(test)]
mod tests {
    use super::*;
    use alloy::{primitives::U256, sol_types::SolValue};
    use std::path::Path;

    #[tokio::test]
    async fn test_risc0_bonsai_worker_execution() {
        // Load .env from workspace root
        dotenv::from_filename("../../.env").expect("Failed to load .env file");

        // Read API key and URL from environment, or skip test if not available
        let (api_key, api_url) = match (
            std::env::var("BONSAI_API_KEY"),
            std::env::var("BONSAI_API_URL"),
        ) {
            (Ok(key), Ok(url)) => (key, url),
            _ => {
                eprintln!("Skipping test: BONSAI_API_KEY and BONSAI_API_URL environment variables must be set");
                return;
            }
        };
        std::env::set_var("BONSAI_API_KEY", api_key);
        std::env::set_var("BONSAI_API_URL", api_url);

        // proof information
        //let risc0_image_id: FixedBytes<32> =
        //    fixed_bytes!("cb7d04f8807ec1b6ffa79c29e4b7c6cb071c1bcc1de2e6c6068882a55ad8f3a8");
        let risc0_guest_program_path = Path::new("../../contracts/test-proof-data/risc0/is-even");

        // proof input
        let proof_input = U256::from(1304);
        let inputs = proof_input.abi_encode();
        // load elf binary
        let elf = std::fs::read(risc0_guest_program_path).unwrap();

        let params = Risc0ProofParams { elf, inputs };

        println!("TEST: generating proof");

        // Call generate_proof
        let snark_receipt = Risc0RemoteProver.generate_proof(&params).await.unwrap();

        println!("receipt: {:?}", snark_receipt);
    }
}*/
