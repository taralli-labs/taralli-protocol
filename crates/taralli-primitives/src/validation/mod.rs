use crate::{
    systems::{ProvingSystem, ProvingSystemId},
    PrimitivesError, Result,
};
use alloy::primitives::{Address, FixedBytes, U256};

pub mod offer;
pub mod request;

// Common validation trait for shared fields across all intent types
pub trait ValidateCommon {
    type System: ProvingSystem;
    fn proving_system_id(&self) -> ProvingSystemId;
    fn proving_system(&self) -> &Self::System;
    fn market_address(&self) -> &Address;
    fn nonce(&self) -> &U256;
    fn start_auction_timestamp(&self) -> u64;
    fn end_auction_timestamp(&self) -> u64;
    fn proving_time(&self) -> u32;
    fn inputs_commitment(&self) -> FixedBytes<32>;
}

// Common validation functions that work with any type implementing ValidateCommon
pub fn validate_proving_system<T: ValidateCommon>(
    compute: &T,
    supported_proving_systems: &[ProvingSystemId],
) -> Result<()> {
    // Check if proving system is supported
    if !supported_proving_systems.contains(&compute.proving_system_id()) {
        return Err(PrimitivesError::ValidationError(
            "proving system id not supported".to_string(),
        ));
    }

    // Validate that the proving system information matches the system ID
    if compute.proving_system().system_id() != compute.proving_system_id() {
        return Err(PrimitivesError::ValidationError(
            "provided proving system does not match system id".to_string(),
        ));
    }

    // Validate the proving system specific parameters
    compute.proving_system().validate_inputs().map_err(|e| {
        PrimitivesError::ValidationError(format!("invalid proving system parameters: {}", e))
    })?;

    Ok(())
}

pub fn validate_market_address<T: ValidateCommon>(
    compute: &T,
    market_address: &Address,
) -> Result<()> {
    if compute.market_address() != market_address {
        return Err(PrimitivesError::ValidationError(
            "market address invalid".to_string(),
        ));
    }
    Ok(())
}

pub fn validate_time_constraints<T: ValidateCommon>(
    compute: &T,
    latest_timestamp: u64,
    minimum_proving_time: u32,
    maximum_start_delay: u32,
) -> Result<()> {
    let start = compute.start_auction_timestamp();
    let end = compute.end_auction_timestamp();

    if latest_timestamp < start.saturating_sub(maximum_start_delay as u64)
        || latest_timestamp >= end
    {
        return Err(PrimitivesError::ValidationError(
            "timestamp invalid: out of bounds".to_string(),
        ));
    }

    if compute.proving_time() < minimum_proving_time {
        return Err(PrimitivesError::ValidationError(
            "proving time invalid: below minimum".to_string(),
        ));
    }

    Ok(())
}

pub fn validate_nonce<T: ValidateCommon>(_compute: &T) -> Result<()> {
    // TODO: Implement nonce validation logic
    Ok(())
}

// Helper function to validate a compute intent's common fields
pub fn validate_common<T: ValidateCommon>(
    compute: &T,
    latest_timestamp: u64,
    market_address: &Address,
    minimum_proving_time: u32,
    maximum_start_delay: u32,
    supported_proving_systems: &[ProvingSystemId],
) -> Result<()> {
    validate_proving_system(compute, supported_proving_systems)?;
    validate_market_address(compute, market_address)?;
    validate_time_constraints(
        compute,
        latest_timestamp,
        minimum_proving_time,
        maximum_start_delay,
    )?;
    validate_nonce(compute)?;
    Ok(())
}

/*/// Validates a request by performing all necessary checks in the correct order
pub fn validate_request<P: ProvingSystem>(
    request: &ComputeRequest<P>,
    latest_timestamp: u64,
    market_address: &Address,
    minimum_proving_time: u32,
    maximum_start_delay: u32,
    maximum_allowed_stake: u128,
    supported_proving_systems: &[ProvingSystemId],
) -> Result<()> {
    validate_proving_system_structure(request, supported_proving_systems)?;
    validate_market_address(request, market_address)?;
    validate_amount_constraints(request, maximum_allowed_stake)?;
    validate_time_constraints(
        request,
        latest_timestamp,
        minimum_proving_time,
        maximum_start_delay,
    )?;
    validate_signature(request)?;
    validate_nonce(request)?;

    Ok(())
}

/// Validates that the request structure matches the claimed proving system
fn validate_proving_system_structure<P: ProvingSystem>(
    request: &ComputeRequest<P>,
    supported_proving_systems: &[ProvingSystemId],
) -> Result<()> {
    // Check if the proving system is supported
    if !supported_proving_systems.contains(&request.proving_system_id) {
        return Err(PrimitivesError::ValidationError(
            "proving system id not supported".to_string(),
        ));
    }

    // Validate that the proving system information matches the system ID
    if request.proving_system.proving_system_id() != request.proving_system_id {
        return Err(PrimitivesError::ValidationError(
            "provided proving system does not match system id".to_string(),
        ));
    }

    // Decode and validate verifier details from the request
    let verifier_details =
        VerifierDetails::abi_decode(&request.onchain_proof_request.extraData, true).map_err(
            |e| {
                PrimitivesError::ValidationError(format!("failed to decode VerifierDetails: {}", e))
            },
        )?;

    // Get the configuration for this proving system
    let config = I::proof_configuration(&request.proving_system);

    // Validate that the verifier details match the constraints for this proving system
    config.validate(&verifier_details).map_err(|e| {
        PrimitivesError::ValidationError(format!(
            "verifier details do not match system constraints: {}",
            e
        ))
    })?;

    // Validate the proving system specific parameters
    request
        .proving_system
        .validate_inputs()
        .map_err(|e| {
            PrimitivesError::ValidationError(format!("invalid proving system parameters: {}", e))
        })?;

    Ok(())
}

pub fn validate_market_address<P: ProvingSystem>(
    request: &ComputeRequest<P>,
    market_address: &Address,
) -> Result<()> {
    if &request.onchain_proof_request.market != market_address {
        Err(PrimitivesError::ValidationError(
            "market address invalid".to_string(),
        ))
    } else {
        Ok(())
    }
}

pub fn validate_amount_constraints<P: ProvingSystem>(
    request: &ComputeRequest<P>,
    maximum_allowed_stake: u128,
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

pub fn validate_time_constraints<P: ProvingSystem>(
    request: &ComputeRequest<P>,
    latest_timestamp: u64,
    minimum_proving_time: u32,
    maximum_start_delay: u32,
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

pub fn validate_nonce<P: ProvingSystem>(_request: &ComputeRequest<P>) -> Result<()> {
    // TODO
    Ok(())
}

*/
