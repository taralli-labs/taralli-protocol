use std::path::PathBuf;

use crate::error::{Result, WorkerError};
use aligned_sdk::core::types::{
    AlignedVerificationData, Network, PriceEstimate, ProvingSystemId, VerificationData,
};
use aligned_sdk::sdk::{estimate_fee, get_nonce_from_ethereum, submit_and_wait_verification};
use async_trait::async_trait;
use risc0_zkvm::ProverOpts;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sp1_sdk::SP1ProofMode;
use taralli_client::error::ClientError;
use taralli_client::worker::{ComputeWorker, WorkResult};
use taralli_primitives::alloy::dyn_abi::dyn_abi::DynSolValue;
use taralli_primitives::alloy::primitives::{Address, Bytes, FixedBytes, U256};
use taralli_primitives::intents::ComputeIntent;
use taralli_primitives::systems::aligned_layer::AlignedLayerProofParams;
use taralli_primitives::systems::gnark::{GnarkConfig, GnarkMode, GnarkProofParams};
use taralli_primitives::systems::sp1::Sp1Config;
use taralli_primitives::systems::{System, SystemParams};
use tempfile::NamedTempFile;

use ethers::core::types::H160;
use ethers::signers::LocalWallet;

use super::risc0::local::Risc0LocalProver;
use super::risc0::Risc0Prover;
use super::sp1::local::Sp1LocalProver;
use super::sp1::Sp1Prover;

const ALIGNED_NETWORK: Network = Network::Holesky;
const BATCHER_URL: &str = "wss://batcher.alignedlayer.com"; // holesky testnet batcher url

#[derive(Debug, Deserialize)]
pub enum AlignedVerificationInputs {
    Gnark {
        config: GnarkConfig,
        proof: Vec<u8>,
        public_inputs: Vec<u8>,
        verification_key: Vec<u8>,
    },
    SP1 {
        config: Sp1Config,
        proof: Vec<u8>,
        vm_program: Vec<u8>,
    },
    Risc0 {
        proof: Vec<u8>,
        vm_program: Vec<u8>,
        public_input: Vec<u8>,
    },
}

pub struct AlignedLayerWorker {
    prover_address: Address,
    rpc_url: String,
    wallet: LocalWallet,
}

impl AlignedLayerWorker {
    pub fn new(prover_address: Address, rpc_url: String, wallet: LocalWallet) -> Self {
        Self {
            prover_address,
            rpc_url,
            wallet,
        }
    }

    fn format_opaque_submission(
        &self,
        aligned_verification_data: AlignedVerificationData,
    ) -> Result<Bytes> {
        let batch_inclusion_proof_bytes: Vec<u8> = aligned_verification_data
            .batch_inclusion_proof
            .merkle_path
            .iter()
            .flat_map(|x| x.as_slice())
            .copied()
            .collect();

        let merkle_proof_input_values = DynSolValue::Tuple(vec![
            DynSolValue::FixedBytes(
                aligned_verification_data
                    .verification_data_commitment
                    .proof_commitment
                    .into(),
                32,
            ),
            DynSolValue::FixedBytes(
                aligned_verification_data
                    .verification_data_commitment
                    .pub_input_commitment
                    .into(),
                32,
            ),
            DynSolValue::FixedBytes(
                aligned_verification_data
                    .verification_data_commitment
                    .proving_system_aux_data_commitment
                    .into(),
                32,
            ),
            DynSolValue::FixedBytes(FixedBytes::from_slice(self.prover_address.as_slice()), 20),
            DynSolValue::FixedBytes(aligned_verification_data.batch_merkle_root.into(), 32),
            DynSolValue::Bytes(batch_inclusion_proof_bytes),
            DynSolValue::Uint(
                U256::from(aligned_verification_data.index_in_batch as u8),
                256,
            ),
            DynSolValue::Address(Address::ZERO),
        ]);

        Ok(Bytes::from(merkle_proof_input_values.abi_encode()))
    }

    fn compute_partial_commitment() -> Result<FixedBytes<32>> {
        Ok(FixedBytes::new([0u8; 32]))
    }

    fn prepare_aligned_verification_data(
        &self,
        inputs: AlignedVerificationInputs,
    ) -> Result<VerificationData> {
        match inputs {
            AlignedVerificationInputs::Gnark {
                config,
                proof,
                public_inputs,
                verification_key,
            } => {
                let proving_system = match config.mode {
                    GnarkMode::Groth16Bn254 => ProvingSystemId::Groth16Bn254,
                    GnarkMode::PlonkBn254 => ProvingSystemId::GnarkPlonkBn254,
                    GnarkMode::PlonkBls12_381 => ProvingSystemId::GnarkPlonkBls12_381,
                };
                Ok(VerificationData {
                    proving_system,
                    proof,
                    proof_generator_addr: H160::from_slice(self.prover_address.as_slice()),
                    vm_program_code: None,
                    verification_key: Some(verification_key),
                    pub_input: Some(public_inputs),
                })
            }
            AlignedVerificationInputs::SP1 {
                config: _,
                proof,
                vm_program,
            } => Ok(VerificationData {
                proving_system: ProvingSystemId::SP1,
                proof,
                proof_generator_addr: H160::from_slice(self.prover_address.as_slice()),
                vm_program_code: Some(vm_program),
                verification_key: None,
                pub_input: None,
            }),
            AlignedVerificationInputs::Risc0 {
                proof,
                vm_program,
                public_input,
            } => Ok(VerificationData {
                proving_system: ProvingSystemId::Risc0,
                proof,
                proof_generator_addr: H160::from_slice(self.prover_address.as_slice()),
                vm_program_code: Some(vm_program),
                verification_key: None,
                pub_input: Some(public_input),
            }),
        }
    }

    async fn execute_gnark_prover(gnark_params: &GnarkProofParams) -> Result<PathBuf> {
        let mut params_file =
            NamedTempFile::new().map_err(|e| WorkerError::ExecutionFailed(e.to_string()))?;
        let proof_output_file =
            NamedTempFile::new().map_err(|e| WorkerError::ExecutionFailed(e.to_string()))?;

        // Create a serializable structure that matches GnarkProofParams
        #[derive(Serialize)]
        struct GnarkProverInput {
            r1cs: Vec<u8>,
            public_inputs: Value,
            private_inputs: Value,
            scheme_config: String,
            curve: String,
        }

        // Build command based on scheme configuration
        let (scheme, curve) = match gnark_params.config.mode {
            GnarkMode::Groth16Bn254 => ("groth16", "bn254"),
            GnarkMode::PlonkBn254 => ("plonk", "bn254"),
            GnarkMode::PlonkBls12_381 => ("plonk", "bls12-381"),
        };

        // Create the input structure
        let prover_input = GnarkProverInput {
            r1cs: gnark_params.r1cs.clone(),
            public_inputs: gnark_params.public_inputs.clone(),
            private_inputs: gnark_params.inputs.clone(),
            scheme_config: scheme.to_string(),
            curve: curve.to_string(),
        };

        // Write all params to a single JSON file
        serde_json::to_writer(&mut params_file, &prover_input)
            .map_err(|e| WorkerError::ExecutionFailed(e.to_string()))?;

        // Execute the Go prover with single input file
        let prover_output = tokio::process::Command::new("gnark-prover")
            .arg("--params")
            .arg(params_file.path())
            .arg("--output")
            .arg(proof_output_file.path())
            .output()
            .await
            .map_err(|e| WorkerError::ExecutionFailed(e.to_string()))?;

        if !prover_output.status.success() {
            return Err(WorkerError::ExecutionFailed(
                String::from_utf8_lossy(&prover_output.stderr).to_string(),
            ));
        }

        Ok(proof_output_file.path().to_path_buf())
    }

    async fn generate_proof(
        &self,
        params: &AlignedLayerProofParams,
    ) -> Result<AlignedVerificationInputs> {
        let aligned_verification_inputs = match params.aligned_proving_system_id.as_str() {
            "Gnark" => {
                // Handle Gnark
                let gnark_params = match *params.config.underlying_system.clone() {
                    SystemParams::Gnark(gnark_params) => gnark_params,
                    _ => {
                        return Err(WorkerError::ExecutionFailed(
                            "Expected Gnark proof params for Gnark proving system".into(),
                        ))
                    }
                };

                // run gnark prover
                let proof_output_path = Self::execute_gnark_prover(&gnark_params).await?;

                // Deserialize proof info from go prover output file
                let aligned_verification_inputs: AlignedVerificationInputs =
                    serde_json::from_reader(
                        std::fs::File::open(proof_output_path)
                            .map_err(|e| WorkerError::ExecutionFailed(e.to_string()))?,
                    )
                    .map_err(|e| {
                        WorkerError::ExecutionFailed(format!(
                            "Failed to parse proof output JSON: {}",
                            e
                        ))
                    })?;
                Ok::<AlignedVerificationInputs, WorkerError>(aligned_verification_inputs)
            }
            "SP1" => {
                let sp1_params = match *params.config.underlying_system.clone() {
                    SystemParams::Sp1(sp1_params) => sp1_params,
                    _ => {
                        return Err(WorkerError::ExecutionFailed(
                            "Expected SP1 proof params for SP1 proving system".into(),
                        ))
                    }
                };

                // Create local SP1 prover with CPU mode
                let sp1_local_prover = Sp1LocalProver::new(false, SP1ProofMode::Groth16);
                // compute sp1 proof locally
                let (sp1_proof, _vk) = sp1_local_prover.generate_proof(&sp1_params).await?;

                // serialize proof for aligned layer
                let serialized_proof = bincode::serialize(&sp1_proof)
                    .map_err(|e| WorkerError::ExecutionFailed(e.to_string()))?;

                Ok(AlignedVerificationInputs::SP1 {
                    config: sp1_params.config.clone(),
                    proof: serialized_proof,
                    vm_program: sp1_params.elf.clone(),
                })
            }
            "Risc0" => {
                let risc0_params = match *params.config.underlying_system.clone() {
                    SystemParams::Risc0(risc0_params) => risc0_params,
                    _ => {
                        return Err(WorkerError::ExecutionFailed(
                            "Expected Risc0 proof params for Risc0 proving system".into(),
                        ))
                    }
                };

                let risc0_prover = Risc0LocalProver::new(ProverOpts::succinct());
                let receipt = risc0_prover.generate_proof(&risc0_params).await?;
                let serialized_proof = bincode::serialize(&receipt.inner)
                    .map_err(|e| WorkerError::ExecutionFailed(e.to_string()))?;

                Ok(AlignedVerificationInputs::Risc0 {
                    proof: serialized_proof,
                    vm_program: risc0_params.elf.clone(),
                    public_input: risc0_params.inputs.clone(),
                })
            }
            _ => {
                return Err(WorkerError::ExecutionFailed(format!(
                    "Unknown proving system: {}",
                    params.aligned_proving_system_id
                )));
            }
        }?;

        Ok(aligned_verification_inputs)
    }

    async fn submit_proof_to_aligned_layer(
        &self,
        inputs: AlignedVerificationInputs,
    ) -> Result<AlignedVerificationData> {
        // prep proof for aligned layer submission
        let verification_data = self.prepare_aligned_verification_data(inputs)?;

        let max_fee = estimate_fee(&self.rpc_url, PriceEstimate::Instant)
            .await
            .expect("failed to fetch gas price from the blockchain");
        let nonce = get_nonce_from_ethereum(
            &self.rpc_url,
            H160::from_slice(self.prover_address.as_slice()),
            ALIGNED_NETWORK,
        )
        .await
        .map_err(|e| WorkerError::ExecutionFailed(format!("{:?}", e)))?;

        // Call batcher through SDK:
        let aligned_layer_submission_output_data = submit_and_wait_verification(
            BATCHER_URL,
            &self.rpc_url,
            ALIGNED_NETWORK,
            &verification_data,
            max_fee,
            self.wallet.clone(),
            nonce,
        )
        .await
        .map_err(|e| WorkerError::ExecutionFailed(e.to_string()))?;
        Ok(aligned_layer_submission_output_data)
    }
}

#[async_trait]
impl<I> ComputeWorker<I> for AlignedLayerWorker
where
    I: ComputeIntent<System = SystemParams> + Send + Sync,
{
    async fn execute(&self, intent: &I) -> taralli_client::error::Result<WorkResult> {
        tracing::info!("aligned layer worker: execution started");

        let system_params = intent
            .system()
            .system_params()
            .ok_or_else(|| ClientError::WorkerError("System params not available".into()))?;

        // prover parameters introspection
        let params = match system_params {
            SystemParams::AlignedLayer(params) => params.clone(),
            _ => {
                return Err(ClientError::WorkerError(
                    "Expected Aligned layer proof params".into(),
                ))
            }
        };

        // generate proof
        let aligned_verification_inputs = self.generate_proof(&params).await.map_err(|e| {
            WorkerError::ExecutionFailed(format!("Failed to generate proof: {}", e))
        })?;

        tracing::info!("Worker finished generating proof, submitting to aligned layer batcher then awaiting batch inclusion");

        let aligned_verification_data = self
            .submit_proof_to_aligned_layer(aligned_verification_inputs)
            .await?;

        tracing::info!(
            "proof successfully included in a valid aligned layer batch, crafting worker result"
        );

        let opaque_submission = self.format_opaque_submission(aligned_verification_data)?;
        let partial_commitment = Self::compute_partial_commitment()?;
        Ok(WorkResult {
            opaque_submission,
            partial_commitment,
        })
    }
}
