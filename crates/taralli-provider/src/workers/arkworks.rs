use crate::{
    error::{ProviderError, Result},
    worker::{ComputeWorker, WorkResult},
};
use ark_bn254::{Bn254, Fr};
use ark_circom::{circom::R1CSFile, CircomCircuit, WitnessCalculator};
use ark_crypto_primitives::snark::SNARK;
use ark_groth16::{Groth16, Proof};
use ark_std::rand::thread_rng;
use async_trait::async_trait;
use num_bigint::BigInt;
use serde_json::Value;
use std::str::FromStr;
use std::{collections::HashMap, fs::File, io::Write};
use taralli_primitives::{
    alloy::{
        dyn_abi::dyn_abi::DynSolValue,
        primitives::{Bytes, FixedBytes, U256},
    },
    taralli_systems::id::ProvingSystemParams,
};
use taralli_primitives::{taralli_systems::systems::arkworks::ArkworksProofParams, Request};
use tempfile::NamedTempFile;
use wasmer::Store;

type TripleOf<T> = (Vec<T>, Vec<Vec<T>>, Vec<T>);

#[derive(Default)]
pub struct ArkworksWorker;

/// TODO: make generic over any circuit
impl ArkworksWorker {
    pub fn new() -> Self {
        Self
    }

    fn proof_to_sol_values(proof: &Proof<Bn254>) -> Result<TripleOf<DynSolValue>> {
        fn to_u256<T: std::fmt::Display>(f: T) -> Result<U256> {
            U256::from_str(&f.to_string())
                .map_err(|e| ProviderError::WorkerExecutionFailed(e.to_string()))
        }

        // Extract proof points
        let (a_points, b_points, c_points) = (
            (proof.a.x, proof.a.y),
            (proof.b.x.c1, proof.b.x.c0, proof.b.y.c1, proof.b.y.c0),
            (proof.c.x, proof.c.y),
        );

        // Convert to U256 values
        let [a_x, a_y] = [to_u256(a_points.0)?, to_u256(a_points.1)?];
        let [b_x_c1, b_x_c0, b_y_c1, b_y_c0] = [
            to_u256(b_points.0)?,
            to_u256(b_points.1)?,
            to_u256(b_points.2)?,
            to_u256(b_points.3)?,
        ];
        let [c_x, c_y] = [to_u256(c_points.0)?, to_u256(c_points.1)?];

        // Create the arrays
        let p_a = vec![DynSolValue::Uint(a_x, 256), DynSolValue::Uint(a_y, 256)];
        let p_b = vec![
            vec![
                DynSolValue::Uint(b_x_c1, 256),
                DynSolValue::Uint(b_x_c0, 256),
            ],
            vec![
                DynSolValue::Uint(b_y_c1, 256),
                DynSolValue::Uint(b_y_c0, 256),
            ],
        ];
        let p_c = vec![DynSolValue::Uint(c_x, 256), DynSolValue::Uint(c_y, 256)];

        Ok((p_a, p_b, p_c))
    }

    fn public_inputs_to_sol_values(public_inputs: &[Fr]) -> Result<Vec<DynSolValue>> {
        fn to_u256<T: std::fmt::Display>(f: T) -> Result<U256> {
            U256::from_str(&f.to_string())
                .map_err(|e| ProviderError::WorkerExecutionFailed(e.to_string()))
        }

        // Convert public inputs to DynSolValues
        public_inputs
            .iter()
            .map(|input| {
                let value = to_u256(input.to_string())?;
                Ok(DynSolValue::Uint(value, 256))
            })
            .collect()
    }

    fn format_opaque_submission(proof: &Proof<Bn254>, public_inputs: &[Fr]) -> Result<Bytes> {
        let (p_a, p_b, p_c) = Self::proof_to_sol_values(proof)?;
        let pub_signals = Self::public_inputs_to_sol_values(public_inputs)?;

        // Create the final tuple for solidity encoding
        let proof_input_values = DynSolValue::Tuple(vec![
            DynSolValue::Array(p_a),
            DynSolValue::Array(vec![
                DynSolValue::Array(p_b[0].clone()),
                DynSolValue::Array(p_b[1].clone()),
            ]),
            DynSolValue::Array(p_c),
            DynSolValue::Array(pub_signals),
        ]);

        Ok(Bytes::from(proof_input_values.abi_encode()))
    }

    fn compute_partial_commitment() -> FixedBytes<32> {
        // Implement commitment computation if needed
        FixedBytes::new([0u8; 32])
    }

    async fn generate_proof(
        &self,
        params: &ArkworksProofParams,
    ) -> Result<(Proof<Bn254>, Vec<Fr>)> {
        // Create temp files for prover inputs and keep them alive for the entire function
        let mut r1cs_file = NamedTempFile::new()
            .map_err(|e| ProviderError::WorkerExecutionFailed(e.to_string()))?;
        let mut wasm_file = NamedTempFile::new()
            .map_err(|e| ProviderError::WorkerExecutionFailed(e.to_string()))?;

        // Write the binary data to temp files
        r1cs_file
            .write_all(&params.r1cs)
            .map_err(|e| ProviderError::WorkerExecutionFailed(e.to_string()))?;
        wasm_file
            .write_all(&params.wasm)
            .map_err(|e| ProviderError::WorkerExecutionFailed(e.to_string()))?;

        let r1cs_path = r1cs_file.path();
        let wasm_path = wasm_file.path();

        // Create a new store for the WASM runtime
        let mut store = Store::default();

        // Initialize the witness calculator with the WASM file
        let mut witness_calculator = WitnessCalculator::new(&mut store, wasm_path)
            .map_err(|e| ProviderError::WorkerExecutionFailed(e.to_string()))?;

        // Convert JSON inputs to the format expected by WitnessCalculator
        let inputs: HashMap<String, Vec<BigInt>> = if let Value::Object(map) = &params.input {
            map.iter()
                .map(|(key, value)| {
                    let values = match value {
                        Value::String(s) => vec![BigInt::parse_bytes(s.as_bytes(), 10)
                            .ok_or_else(|| {
                                ProviderError::WorkerExecutionFailed(
                                    "Invalid input format".to_string(),
                                )
                            })?],
                        Value::Number(n) => vec![BigInt::from(n.as_i64().ok_or_else(|| {
                            ProviderError::WorkerExecutionFailed(
                                "Invalid number format".to_string(),
                            )
                        })?)],
                        _ => {
                            return Err(ProviderError::WorkerExecutionFailed(
                                "Invalid input type".to_string(),
                            ))
                        }
                    };
                    Ok((key.clone(), values))
                })
                .collect::<Result<HashMap<_, _>>>()?
        } else {
            return Err(ProviderError::WorkerExecutionFailed(
                "Invalid input format".to_string(),
            ));
        };

        // Calculate the witness
        let witness = witness_calculator
            .calculate_witness_element::<Fr, _>(&mut store, inputs, false)
            .map_err(|e| ProviderError::WorkerExecutionFailed(e.to_string()))?;

        // Create circuit instance with the calculated witness
        let circuit = CircomCircuit::<Fr> {
            r1cs: R1CSFile::new(
                File::open(r1cs_path)
                    .map_err(|e| ProviderError::WorkerExecutionFailed(e.to_string()))?,
            )
            .map_err(|e| ProviderError::WorkerExecutionFailed(e.to_string()))?
            .into(),
            witness: Some(witness.clone()),
        };

        // Generate parameters and create proof
        let mut rng = thread_rng();
        let params =
            Groth16::<Bn254>::generate_random_parameters_with_reduction(circuit.clone(), &mut rng)
                .map_err(|e| ProviderError::WorkerExecutionFailed(e.to_string()))?;

        let proof = Groth16::<Bn254>::prove(&params, circuit.clone(), &mut rng)
            .map_err(|e| ProviderError::WorkerExecutionFailed(e.to_string()))?;

        // Get public inputs from the witness (typically the first elements)
        let public_inputs = circuit.get_public_inputs().ok_or_else(|| {
            ProviderError::WorkerExecutionFailed("Failed to get public inputs".to_string())
        })?;

        Ok((proof, public_inputs))
    }
}

#[async_trait]
impl ComputeWorker for ArkworksWorker {
    async fn execute(&self, request: &Request<ProvingSystemParams>) -> Result<WorkResult> {
        tracing::info!("arkworks worker: execution started");

        let params = match &request.proving_system_information {
            ProvingSystemParams::Arkworks(params) => params.clone(),
            _ => {
                return Err(ProviderError::WorkerExecutionFailed(
                    "Expected Arkworks params".into(),
                ))
            }
        };

        // Generate proof
        let (proof, public_inputs) = self.generate_proof(&params).await?;
        // Format proof data for resolution
        let opaque_submission = Self::format_opaque_submission(&proof, &public_inputs)?;
        // get empty partial commitment
        let partial_commitment = Self::compute_partial_commitment();
        Ok(WorkResult {
            opaque_submission,
            partial_commitment,
        })
    }
}
