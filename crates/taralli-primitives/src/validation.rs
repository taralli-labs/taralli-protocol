use crate::{
    abi::universal_bombetta::VerifierDetails,
    request::Request,
    utils::{compute_permit2_digest, compute_request_witness},
    PrimitivesError, Result,
};
use alloy::{primitives::Address, sol_types::SolValue};
use taralli_systems::{id::ProvingSystemId, ProvingSystemInformation};

pub fn validate_proving_system_id<I: ProvingSystemInformation>(
    request: &Request<I>,
    proving_system_ids: Vec<ProvingSystemId>,
) -> Result<()> {
    if !proving_system_ids.contains(&request.proving_system_id) {
        Err(PrimitivesError::ValidationError(
            "proving system id invalid".to_string(),
        ))
    } else {
        Ok(())
    }
}

pub fn validate_proving_system_information<I: ProvingSystemInformation>(
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

pub fn validate_verification_commitments<I: ProvingSystemInformation>(
    request: &Request<I>,
) -> Result<()> {
    let verifier_details =
        VerifierDetails::abi_decode(&request.onchain_proof_request.extraData, true).map_err(
            |e| {
                PrimitivesError::ValidationError(format!("failed to decode VerifierDetails: {}", e))
            },
        )?;

    if I::verifier_constraints() == verifier_details {
        Ok(())
    } else {
        Err(PrimitivesError::ValidationError(
            "verifier details mismatch".to_string(),
        ))
    }
}

pub fn validate_nonce<I: ProvingSystemInformation>(_request: &Request<I>) -> Result<()> {
    // TODO
    Ok(())
}

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
