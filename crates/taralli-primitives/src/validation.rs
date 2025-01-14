use crate::{
    abi::universal_bombetta::VerifierDetails,
    request::ProofRequest,
    utils::{compute_permit2_digest, compute_request_witness},
    PrimitivesError, Result,
};
use alloy::{primitives::Address, sol_types::SolValue};
use taralli_systems::{id::ProvingSystemId, ProvingSystemInformation};

pub fn validate_proving_system_id<I: ProvingSystemInformation>(
    request: &ProofRequest<I>,
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
    request: &ProofRequest<I>,
) -> Result<()> {
    request
        .proving_system_information
        .validate_prover_inputs()
        .map_err(|e| PrimitivesError::ValidationError(e.to_string()))
}

pub fn validate_market_address<I: ProvingSystemInformation>(
    request: &ProofRequest<I>,
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

/*pub fn validate_verification_commitments<I: ProvingSystemInformation>(
    request: &ProofRequest<I>,
) -> Result<()> {
    let verifier_details =
        VerifierDetails::abi_decode(&request.onchain_proof_request.extraData, true).map_err(
            |e| {
                PrimitivesError::ValidationError(format!("failed to decode VerifierDetails: {}", e))
            },
        )?;

    let verifier_constraints = I::verifier_constraints();

    if let Some(verifier) = verifier_constraints.verifier {
        if verifier_details.verifier != verifier {
            return Err(PrimitivesError::ValidationError(
                "verifier address mismatch".to_string(),
            ));
        }
    }

    if let Some(selector) = verifier_constraints.selector {
        if verifier_details.selector != selector {
            return Err(PrimitivesError::ValidationError(
                "selector mismatch".to_string(),
            ));
        }
    }

    if let Some(is_sha) = verifier_constraints.is_sha_commitment {
        if verifier_details.isShaCommitment != is_sha {
            return Err(PrimitivesError::ValidationError(
                "isShaCommitment mismatch".to_string(),
            ));
        }
    }

    if let Some(offset) = verifier_constraints.public_inputs_offset {
        if verifier_details.publicInputsOffset != offset {
            return Err(PrimitivesError::ValidationError(
                "publicInputsOffset mismatch".to_string(),
            ));
        }
    }

    if let Some(length) = verifier_constraints.public_inputs_length {
        if verifier_details.publicInputsLength != length {
            return Err(PrimitivesError::ValidationError(
                "publicInputsLength mismatch".to_string(),
            ));
        }
    }

    if let Some(has_partial_commitment_check) =
        verifier_constraints.has_partial_commitment_result_check
    {
        if verifier_details.hasPartialCommitmentResultCheck != has_partial_commitment_check {
            return Err(PrimitivesError::ValidationError(
                "hasPartialCommitmentResultCheck mismatch".to_string(),
            ));
        }
    }

    if let Some(submitted_partial_commitment_offset) =
        verifier_constraints.submitted_partial_commitment_result_offset
    {
        if verifier_details.submittedPartialCommitmentResultOffset
            != submitted_partial_commitment_offset
        {
            return Err(PrimitivesError::ValidationError(
                "submittedPartialCommitmentResultOffset mismatch".to_string(),
            ));
        }
    }

    if let Some(submitted_partial_commitment_length) =
        verifier_constraints.submitted_partial_commitment_result_offset
    {
        if verifier_details.submittedPartialCommitmentResultLength
            != submitted_partial_commitment_length
        {
            return Err(PrimitivesError::ValidationError(
                "submittedPartialCommitmentResultLength mismatch".to_string(),
            ));
        }
    }

    if let Some(predetermined_partial_commitment) =
        verifier_constraints.predetermined_partial_commitment
    {
        if verifier_details.predeterminedPartialCommitment != predetermined_partial_commitment {
            return Err(PrimitivesError::ValidationError(
                "predeterminedPartialCommitment mismatch".to_string(),
            ));
        }
    }

    Ok(())
}*/

pub fn validate_verification_commitments<I: ProvingSystemInformation>(
    request: &ProofRequest<I>,
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

pub fn validate_nonce<I: ProvingSystemInformation>(_request: &ProofRequest<I>) -> Result<()> {
    // TODO
    Ok(())
}

pub fn validate_amount_constraints<I: ProvingSystemInformation>(
    maximum_allowed_stake: u128,
    request: &ProofRequest<I>,
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
    request: &ProofRequest<I>,
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

pub fn validate_signature<I: ProvingSystemInformation>(request: &ProofRequest<I>) -> Result<()> {
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
