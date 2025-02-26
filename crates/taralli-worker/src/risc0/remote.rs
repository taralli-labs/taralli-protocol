use std::time::Duration;

use super::Risc0Prover;
use crate::error::{Result, WorkerError};
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
            .map_err(|e| WorkerError::ExecutionFailed(e.to_string()))?;

        // Compute and upload image ID
        let image_id = hex::encode(
            compute_image_id(&program).map_err(|e| WorkerError::ExecutionFailed(e.to_string()))?,
        );

        client
            .upload_img(&image_id, program)
            .await
            .map_err(|e| WorkerError::ExecutionFailed(e.to_string()))?;

        // Prepare and upload input data
        let input_data =
            to_vec(&inputs).map_err(|e| WorkerError::ExecutionFailed(e.to_string()))?;
        let input_data = bytemuck::cast_slice(&input_data).to_vec();
        let input_id = client
            .upload_input(input_data)
            .await
            .map_err(|e| WorkerError::ExecutionFailed(e.to_string()))?;

        // Create session
        let session = client
            .create_session(image_id.clone(), input_id, vec![], false)
            .await
            .map_err(|e| WorkerError::ExecutionFailed(e.to_string()))?;

        // Poll for STARK proof completion
        let _receipt_url = loop {
            let res = session
                .status(&client)
                .await
                .map_err(|e| WorkerError::ExecutionFailed(e.to_string()))?;

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
                        WorkerError::ExecutionFailed(
                            "API error: missing receipt on completed session".into(),
                        )
                    })?;
                }
                _ => {
                    return Err(WorkerError::ExecutionFailed(format!(
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
            .map_err(|e| WorkerError::ExecutionFailed(e.to_string()))?;

        // Poll for SNARK proof completion
        let snark_receipt = loop {
            let res = snark_session
                .status(&client)
                .await
                .map_err(|e| WorkerError::ExecutionFailed(e.to_string()))?;

            match res.status.as_str() {
                "RUNNING" => {
                    tracing::info!("Bonsai SNARK proof status: {}", res.status);
                    tokio::time::sleep(Duration::from_secs(15)).await;
                    continue;
                }
                "SUCCEEDED" => {
                    let receipt_buf = client
                        .download(&res.output.ok_or_else(|| {
                            WorkerError::ExecutionFailed("Missing SNARK output URL".into())
                        })?)
                        .await
                        .map_err(|e| WorkerError::ExecutionFailed(e.to_string()))?;

                    break bincode::deserialize(&receipt_buf)
                        .map_err(|e| WorkerError::ExecutionFailed(e.to_string()))?;
                }
                _ => {
                    return Err(WorkerError::ExecutionFailed(format!(
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
