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

        // Print RISC0 version info
        println!("TEST: locally Using RISC0 version: {}", risc0_zkvm::VERSION);

        // Create async client
        let client = Client::from_env(risc0_zkvm::VERSION)
            .map_err(|e| ProviderError::WorkerExecutionFailed(e.to_string()))?;

        let bonsai_api_version_info = client.version().await.unwrap();
        println!(
            "bonsai api version info: {:?}",
            bonsai_api_version_info.risc0_zkvm
        );

        // Compute and upload image ID
        let image_id = hex::encode(
            compute_image_id(&program)
                .map_err(|e| ProviderError::WorkerExecutionFailed(e.to_string()))?,
        );
        println!("computed image id: {}", image_id);

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

        println!("session created");

        // Poll for STARK proof completion
        let _receipt_url = loop {
            let res = session
                .status(&client)
                .await
                .map_err(|e| ProviderError::WorkerExecutionFailed(e.to_string()))?;

            println!("0");

            match res.status.as_str() {
                "RUNNING" => {
                    println!("1");
                    tracing::info!(
                        "Bonsai STARK proof status: {} - state: {}",
                        res.status,
                        res.state.unwrap_or_default()
                    );
                    tokio::time::sleep(Duration::from_secs(15)).await;
                    continue;
                }
                "SUCCEEDED" => {
                    println!("2");
                    break res.receipt_url.ok_or_else(|| {
                        ProviderError::WorkerExecutionFailed(
                            "API error: missing receipt on completed session".into(),
                        )
                    })?;
                }
                _ => {
                    println!("3");
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
                    tokio::time::sleep(Duration::from_secs(30)).await;
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
    use risc0_zkvm::serde::to_vec;

    #[tokio::test]
    async fn test_risc0_bonsai_worker_execution() {
        // Load .env from workspace root
        dotenv::from_filename("../../.env").expect("Failed to load .env file");
        std::env::var("BONSAI_API_URL").expect("BONSAI_API_URL not set after loading .env");
        std::env::var("BONSAI_API_KEY").expect("BONSAI_API_KEY not set after loading .env");

        // program
        let risc0_guest_program_path = Path::new("../../contracts/test-proof-data/risc0/is-even");
        // proof input
        let proof_input = U256::from(2);
        let inputs = proof_input.abi_encode();
        println!("TEST: Input value: {}", proof_input);
        println!("TEST: Encoded input size: {} bytes", inputs.len());
        println!("TEST: Encoded input (hex): 0x{}", hex::encode(&inputs));

        // load elf binary
        let elf = std::fs::read(risc0_guest_program_path).unwrap();

        let params = Risc0ProofParams { elf, inputs };

        // Call generate_proof
        let snark_receipt = Risc0RemoteProver.generate_proof(&params).await.unwrap();

        println!("receipt: {:?}", snark_receipt);
    }

    #[tokio::test]
    async fn test_bonsai_sdk() {
        // Enable logging
        tracing_subscriber::fmt::init();
        // Load environment
        dotenv::from_filename("../../.env").expect("Failed to load .env file");
        // Debug: Print environment variables
        let api_url = std::env::var("BONSAI_API_URL")
            .expect("BONSAI_API_URL not set after loading .env");
        let api_key = std::env::var("BONSAI_API_KEY")
            .expect("BONSAI_API_KEY not set after loading .env");

        let client = Client::from_env(risc0_zkvm::VERSION).expect("failed to set client from env");

        // Make raw request to see actual response
        let response = reqwest::Client::new()
            .get(format!("{}/user/quotas", api_url))
            .header("x-api-key", api_key)
            .header("x-risc0-version", risc0_zkvm::VERSION)
            .send()
            .await
            .expect("Failed to make request");

        let status = response.status();
        let body = response.text().await.expect("Failed to get response text");

        println!("Response Status: {}", status);
        println!("Response Body: {}", body);

        // program
        let risc0_guest_program_path = Path::new("../../contracts/test-proof-data/risc0/is-even");
        let elf = std::fs::read(risc0_guest_program_path).expect("failed to read elf");
        // proof input
        let proof_input = U256::from(2);
        let input_bytes = proof_input.abi_encode();
        let input_data = bytemuck::cast_slice(&input_bytes).to_vec();
        println!("TEST: Input value: {}", proof_input);

        // Compute image ID
        let image_id = hex::encode(compute_image_id(&elf).expect("Failed to compute image ID"));
        println!("TEST: Image ID: {}", image_id);

        // Upload ELF
        println!("TEST: Uploading ELF...");
        client.upload_img(&image_id, elf.clone())
            .await
            .expect("Failed to upload ELF");

        println!("TEST: Uploading input (hex: 0x{})...", hex::encode(&input_bytes));
        let input_id = client.upload_input(input_data)
            .await
            .expect("Failed to upload input");

        // Create STARK proving session
        println!("TEST: Creating STARK proving session...");
        let session = client
            .create_session(image_id.clone(), input_id, vec![], false)
            .await
            .expect("Failed to create session");
        println!("TEST: Session UUID: {}", session.uuid);

        // Wait for STARK proof
        println!("TEST: Waiting for STARK proof...");
        let receipt_url = loop {
            let status = session.status(&client)
                .await
                .expect("Failed to get session status");

            println!("TEST: STARK Status: {} (State: {:?})",
                status.status,
                status.state.unwrap_or_default()
            );

            match status.status.as_str() {
                "RUNNING" => {
                    tokio::time::sleep(Duration::from_secs(15)).await;
                    continue;
                }
                "SUCCEEDED" => {
                    break status.receipt_url.expect("Missing receipt URL");
                }
                _ => {
                    panic!("STARK proof failed: {} (Error: {})",
                        status.status,
                        status.error_msg.unwrap_or_default()
                    );
                }
            }
        };

        // Download STARK receipt
        println!("TEST: Downloading STARK receipt...");
        let _stark_receipt = client.download(&receipt_url)
            .await
            .expect("Failed to download STARK receipt");

        // Create SNARK proof
        println!("TEST: Creating SNARK proof...");
        let snark_session = client.create_snark(session.uuid)
            .await
            .expect("Failed to create SNARK session");
        println!("TEST: SNARK Session UUID: {}", snark_session.uuid);

        // Wait for SNARK proof
        println!("TEST: Waiting for SNARK proof...");
        let snark_receipt: Receipt = loop {
            let status = snark_session.status(&client)
                .await
                .expect("Failed to get SNARK status");

            println!("TEST: SNARK Status: {}", status.status);

            match status.status.as_str() {
                "RUNNING" => {
                    tokio::time::sleep(Duration::from_secs(15)).await;
                    continue;
                }
                "SUCCEEDED" => {
                    let output_url = status.output.expect("Missing SNARK output URL");
                    let receipt_bytes = client.download(&output_url)
                        .await
                        .expect("Failed to download SNARK receipt");
                    break bincode::deserialize(&receipt_bytes)
                        .expect("Failed to deserialize SNARK receipt");
                }
                _ => {
                    panic!("SNARK proof failed: {} (Error: {})",
                        status.status,
                        status.error_msg.unwrap_or_default()
                    );
                }
            }
        };

        println!("TEST: Successfully generated STARK and SNARK proofs!");
        println!("TEST: Final receipt: {:?}", snark_receipt);
    }

    #[tokio::test]
    async fn test_blocking_bonsai() -> Result<()> {
        // Enable logging
        tracing_subscriber::fmt::init();
        // Load environment
        dotenv::from_filename("../../.env").expect("Failed to load .env file");
        std::env::var("BONSAI_API_URL").expect("BONSAI_API_URL not set after loading .env");
        std::env::var("BONSAI_API_KEY").expect("BONSAI_API_KEY not set after loading .env");

        let client = Client::from_env(risc0_zkvm::VERSION).expect("client env failed");

        // program
        let risc0_guest_program_path = Path::new("../../contracts/test-proof-data/risc0/is-even");
        let elf = std::fs::read(risc0_guest_program_path).expect("failed to read elf");
        // proof input
        let proof_input = U256::from(2);
        let input_bytes = proof_input.abi_encode();
        let input_data: Vec<u32> = bytemuck::cast_slice(&input_bytes).to_vec();
        println!("TEST: Input value: {}", proof_input);

        // Compute image ID
        let image_id = hex::encode(compute_image_id(&elf).expect("Failed to compute image ID"));
        println!("TEST: Image ID: {}", image_id);

        // Prepare input data and upload it.
        let input_data = to_vec(&input_data).unwrap();
        let input_data = bytemuck::cast_slice(&input_data).to_vec();
        let input_id = client.upload_input(input_data).await.expect("upload input failed");

        // Add a list of assumptions
        let assumptions: Vec<String> = vec![];

        // Wether to run in execute only mode
        let execute_only = false;

        // Start a session running the prover
        let session = client.create_session(image_id.clone(), input_id, assumptions, execute_only).await.expect("create session failed");
        loop {
            let res = session.status(&client).await.expect("session status failed");
            if res.status == "RUNNING" {
                eprintln!(
                    "Current status: {} - state: {} - continue polling...",
                    res.status,
                    res.state.unwrap_or_default()
                );
                std::thread::sleep(Duration::from_secs(15));
                continue;
            }
            if res.status == "SUCCEEDED" {
                // Download the receipt, containing the output
                let receipt_url = res
                    .receipt_url
                    .expect("API error, missing receipt on completed session");

                let receipt_buf = client.download(&receipt_url).await.expect("download failed");
                let receipt: Receipt = bincode::deserialize(&receipt_buf).expect("desrialize fucked");
                let image_bytes: [u8; 32] = hex::decode(image_id.clone()).expect("Invalid hex").try_into().expect("Wrong length");
                receipt
                    .verify(image_bytes)
                    .expect("Receipt verification failed");
            } else {
                panic!(
                    "Workflow exited: {} - | err: {}",
                    res.status,
                    res.error_msg.unwrap_or_default()
                );
            }
        }

        // stark2snark
        // run_stark2snark(session.uuid)?;

    }

    #[tokio::test]
    async fn test_bonsai_quotas() {
        // Load environment
        dotenv::from_filename("../../.env").expect("Failed to load .env file");
        std::env::var("BONSAI_API_URL").expect("BONSAI_API_URL not set after loading .env");
        std::env::var("BONSAI_API_KEY").expect("BONSAI_API_KEY not set after loading .env");

        // Create client using the SDK
        let client = Client::from_env(risc0_zkvm::VERSION)
            .expect("Failed to create Bonsai client");

        // Fetch quotas using the SDK
        match client.quotas().await {
            Ok(quotas) => {
                println!("\nBonsai Quota Information:");
                println!("-------------------------");
                println!("Executor cycle limit: {}", quotas.exec_cycle_limit);
                println!("Concurrent proofs: {}", quotas.concurrent_proofs);
                println!("Cycle budget remaining: {}", quotas.cycle_budget);
                println!("Lifetime cycles used: {}", quotas.cycle_usage);
                println!("Dedicated executor: {}", quotas.dedicated_executor);
                println!("Dedicated GPU: {}", quotas.dedicated_gpu);
            }
            Err(e) => {
                panic!("Failed to fetch Bonsai quotas: {:?}", e);
            }
        }
    }
}*/
