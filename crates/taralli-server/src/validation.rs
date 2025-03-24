use crate::{
    error::{Result, ServerError},
    state::{offer::OfferState, request::RequestState},
};
use taralli_primitives::{
    alloy::{
        eips::BlockId,
        network::{BlockTransactionsKind, Ethereum},
        providers::Provider,
        transports::Transport,
    },
    compression_utils::intents::{PartialComputeOffer, PartialComputeRequest},
    validation::{
        offer::{validate_offer_amount_constraints, validate_offer_signature},
        request::{validate_request_amount_constraints, validate_request_signature},
        validate_market_address, validate_time_constraints,
    },
};

/// Validate a submitted compute intent
pub async fn validate_partial_request<T: Transport + Clone, P: Provider<T> + Clone>(
    partial_request: &PartialComputeRequest,
    state: &RequestState<T, P>,
) -> Result<()> {
    // TODO: separate this timestamp fetch from the validation execution of the server
    #[cfg(not(feature = "ci-test"))]
    let latest_timestamp = get_latest_timestamp(state.rpc_provider()).await?;

    // We have some tests for the transport of data between submit/subscribe.
    // Since said tests are carried by communicating with the deployed binary of the server, mocking this function
    // is only possible via feature flags.
    #[cfg(feature = "ci-test")]
    let latest_timestamp = partial_request.proof_request.startAuctionTimestamp
        - state
            .base
            .validation_configs()
            .request
            .base
            .maximum_start_delay as u64;

    let config = &state.validation_configs().request;

    // check system id exists, skip full system validation
    if !config
        .base
        .supported_systems
        .contains(&partial_request.system_id)
    {
        return Err(ServerError::ValidationError("unsupported system id".into()));
    }

    // complete partial valiation of non compressed fields in the intent
    validate_market_address(
        &partial_request.proof_request.market,
        &state.universal_bombetta_address(),
    )?;
    validate_request_amount_constraints(
        &partial_request.proof_request,
        config.maximum_allowed_stake,
    )?;
    validate_time_constraints(
        partial_request.proof_request.startAuctionTimestamp,
        partial_request.proof_request.endAuctionTimestamp,
        partial_request.proof_request.provingTime,
        latest_timestamp,
        config.base.minimum_proving_time,
        config.base.maximum_start_delay,
    )?;
    validate_request_signature(&partial_request.proof_request, &partial_request.signature)?;

    Ok(())
}

pub async fn validate_partial_offer<T: Transport + Clone, P: Provider<T> + Clone>(
    partial_offer: &PartialComputeOffer,
    state: &OfferState<T, P>,
) -> Result<()> {
    // TODO: separate this timestamp fetch from the validation execution of the server
    #[cfg(not(feature = "ci-test"))]
    let latest_timestamp = get_latest_timestamp(state.rpc_provider()).await?;

    // We have some tests for the transport of data between submit/subscribe.
    // Since said tests are carried by communicating with the deployed binary of the server, mocking this function
    // is only possible via feature flags.
    #[cfg(feature = "ci-test")]
    let latest_timestamp = partial_offer.proof_offer.startAuctionTimestamp
        - state
            .base
            .validation_configs()
            .offer
            .base
            .maximum_start_delay as u64;

    let config = &state.validation_configs().offer;

    // check system id exists, skip full system validation
    if !config
        .base
        .supported_systems
        .contains(&partial_offer.system_id)
    {
        return Err(ServerError::ValidationError("unsupported system id".into()));
    }

    // complete partial valiation of non compressed fields in the intent
    validate_market_address(
        &partial_offer.proof_offer.market,
        &state.universal_porchetta_address(),
    )?;
    validate_offer_amount_constraints(
        &partial_offer.proof_offer,
        config.maximum_allowed_reward,
        config.minimum_allowed_stake,
    )?;
    validate_time_constraints(
        partial_offer.proof_offer.startAuctionTimestamp,
        partial_offer.proof_offer.endAuctionTimestamp,
        partial_offer.proof_offer.provingTime,
        latest_timestamp,
        config.base.minimum_proving_time,
        config.base.maximum_start_delay,
    )?;
    validate_offer_signature(&partial_offer.proof_offer, &partial_offer.signature)?;

    Ok(())
}

#[allow(dead_code)]
async fn get_latest_timestamp<P: Provider<T, Ethereum> + Clone, T: Transport + Clone>(
    provider: P,
) -> Result<u64> {
    provider
        .get_block(BlockId::latest(), BlockTransactionsKind::Hashes)
        .await
        .map_err(|_| ServerError::FetchLatestBlockTimestampError)?
        .map(|block| block.header.timestamp)
        .ok_or(ServerError::FetchLatestBlockTimestampError)
}
