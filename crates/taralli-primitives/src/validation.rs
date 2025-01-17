use crate::{
    abi::universal_bombetta::VerifierDetails, request::Request, systems::{ProofConfiguration, ProvingSystemId, ProvingSystemInformation, ProvingSystemParams}, utils::{compute_permit2_digest, compute_request_witness}, PrimitivesError, Result
};
use alloy::{primitives::Address, sol_types::SolValue};

/// Validates a request by performing all necessary checks in the correct order
pub fn validate_request<I: ProvingSystemInformation>(
    request: &Request<I>,
    latest_timestamp: u64,
    minimum_proving_time: u32,
    maximum_start_delay: u32,
    maximum_allowed_stake: u128,
    supported_proving_systems: &[ProvingSystemId],
) -> Result<()> {
    // validate request structure matches the claimed proving system
    validate_proving_system_structure(request, supported_proving_systems)?;
    // Then validate all other constraints
    validate_amount_constraints(maximum_allowed_stake, request)?;
    validate_time_constraints(latest_timestamp, minimum_proving_time, maximum_start_delay, request)?;
    validate_signature(request)?;
    validate_nonce(request)?;

    Ok(())
}

/// Validates that the request structure matches the claimed proving system
fn validate_proving_system_structure<I: ProvingSystemInformation>(
    request: &Request<I>,
    supported_proving_systems: &[ProvingSystemId],
) -> Result<()> {
    // Check if the proving system is supported
    if !supported_proving_systems.contains(&request.proving_system_id) {
        return Err(PrimitivesError::ValidationError(
            "proving system id not supported".to_string(),
        ));
    }

    // Validate that the proving system information matches the system ID
    if request.proving_system_information.proving_system_id() != request.proving_system_id {
        return Err(PrimitivesError::ValidationError(
            "proving system information does not match system id".to_string(),
        ));
    }

    // Decode and validate verifier details from the request
    let verifier_details = VerifierDetails::abi_decode(&request.onchain_proof_request.extraData, true)
        .map_err(|e| {
            PrimitivesError::ValidationError(format!("failed to decode VerifierDetails: {}", e))
        })?;

    // Get the configuration for this proving system
    let config = I::proof_configuration(&request.proving_system_information);
    
    // Validate that the verifier details match the constraints for this proving system
    config.validate(&verifier_details)
        .map_err(|e| PrimitivesError::ValidationError(
            format!("verifier details do not match system constraints: {}", e)
        ))?;

    // Validate the proving system specific parameters
    request.proving_system_information.validate_inputs()
        .map_err(|e| PrimitivesError::ValidationError(
            format!("invalid proving system parameters: {}", e)
        ))?;

    Ok(())
}

/*pub fn validate_proving_system_information<I: ProvingSystemInformation>(
    request: &Request<I>,
) -> Result<()> {
    request
        .proving_system_information
        .validate_prover_inputs()
        .map_err(|e| PrimitivesError::ValidationError(e.to_string()))
}

pub fn validate_market_address<I: ProvingSystemInformation>(
    request: &Request<I>,
    market_address: Address,
) -> Result<()> {
    if request.onchain_proof_request.market != market_address {
        Err(PrimitivesError::ValidationError(
            "market address invalid".to_string(),
        ))
    } else {
        Ok(())
    }
}

pub fn validate_verifier_constraints<I: ProvingSystemInformation>(
    request: &Request<I>,
) -> Result<()> {
    let verifier_details =
        VerifierDetails::abi_decode(&request.onchain_proof_request.extraData, true).map_err(
            |e| {
                PrimitivesError::ValidationError(format!("failed to decode VerifierDetails: {}", e))
            },
        )?;

    let config = I::proof_configuration(&request.proving_system_information);
    let is_valid = config.validate(&verifier_details).map_err(|e| PrimitivesError::ValidationError(format!("verifier details do not adhere to constraints: {}", e)))?;
    Ok(is_valid)
}

pub fn validate_nonce<I: ProvingSystemInformation>(_request: &Request<I>) -> Result<()> {
    // TODO
    Ok(())
}*/

pub fn validate_amount_constraints<I: ProvingSystemInformation>(
    maximum_allowed_stake: u128,
    request: &Request<I>,
) -> Result<()> {
    if request.onchain_proof_request.maxRewardAmount < request.onchain_proof_request.minRewardAmount
    {
        Err(PrimitivesError::ValidationError(
            "token amounts invalid".to_string(),
        ))
    } else if request.onchain_proof_request.minimumStake > maximum_allowed_stake {
        Err(PrimitivesError::ValidationError(
            "eth stake amount invalid".to_string(),
        ))
    } else {
        Ok(())
    }
}

pub fn validate_time_constraints<I: ProvingSystemInformation>(
    latest_timestamp: u64,
    minimum_proving_time: u32,
    maximum_start_delay: u32,
    request: &Request<I>,
) -> Result<()> {
    let start = request.onchain_proof_request.startAuctionTimestamp;
    let end = request.onchain_proof_request.endAuctionTimestamp;
    if latest_timestamp < start - maximum_start_delay as u64 || latest_timestamp >= end {
        Err(PrimitivesError::ValidationError(
            "timestamp invalid: out of bounds".to_string(),
        ))
    } else if request.onchain_proof_request.provingTime < minimum_proving_time {
        Err(PrimitivesError::ValidationError(
            "proving time invalid: below minimum".to_string(),
        ))
    } else {
        Ok(())
    }
}

pub fn validate_nonce<I: ProvingSystemInformation>(_request: &Request<I>) -> Result<()> {
    // TODO
    Ok(())
}


pub fn validate_signature<I: ProvingSystemInformation>(request: &Request<I>) -> Result<()> {
    // compute witness
    let witness = compute_request_witness(&request.onchain_proof_request);
    // compute permit digest
    let computed_digest = compute_permit2_digest(&request.onchain_proof_request, witness);
    // ec recover signing public key
    let computed_verifying_key = request
        .signature
        .recover_from_prehash(&computed_digest)
        .map_err(|e| PrimitivesError::ValidationError(format!("ec recover failed: {}", e)))?;
    let computed_signer = Address::from_public_key(&computed_verifying_key);

    // check signature validity
    if computed_signer != request.onchain_proof_request.signer {
        Err(PrimitivesError::ValidationError(
            "signature invalid: computed signer != request.signer".to_string(),
        ))
    } else {
        Ok(())
    }
}