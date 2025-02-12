// SPDX-License-Identifier: MIT
pragma solidity ^0.8.23;

/// @notice Represents an opqaue offer for a proof to be generated
/// @custom:field signer The signer of the signature.
/// @custom:field market The address of the market that the proof offer is being submitted to.
/// @custom:field nonce The permit signature nonce.
/// @custom:field rewardToken The address of the ERC20 token that the offer creator/signer will be rewarded for
///                           resolving the offer.
/// @custom:field rewardAmount The price in tokens the proof provider will be paid
///               upon successfully resolving the proof offer.
/// @custom:field stakeToken The token that will be used as collateral for the proof offer.
/// @custom:field stakeAmount The amount of stake denominated in `stakeToken` that the proof provider is required
///                           to provide as collateral when a requester bids on their proof offer. This value is
///                           slashed if the proof offer is not resolved before the resolution deadline.
/// @custom:field startAuctionTimestamp The starting timestamp of the proof offer auction.
/// @custom:field endAuctionTimestamp The deadline timestamp of the proof offer auction & signature. If no bid is
///                                   submitted before this timestamp, the offer is revoked.
/// @custom:field provingTime Amount of time (in seconds) to generate the proof. Time period starts when proof
///                           offer auction is completed during bombetta.bid()'s execution.
/// @custom:field inputsCommitment Hash of all information the proof provider commits to in order to prove to the
///                                requesting party that they computed the proof that was promised in the offer.
///                                (for example the public inputs of the offered proof)
/// @custom:field extraData Opaque data relating to the Porchetta market specific verification logic, if needed
///                         (verifier addr, fn selector, public inputs calldata location in opaqueSubmission, etc.)
struct ProofOffer {
    // general
    address signer;
    address market;
    uint256 nonce;
    // price
    address rewardToken;
    uint256 rewardAmount;
    // stake requirements
    address stakeToken;
    uint256 stakeAmount;
    // time constraints
    uint64 startAuctionTimestamp;
    uint64 endAuctionTimestamp;
    uint32 provingTime;
    // verification commitments
    bytes32 inputsCommitment;
    bytes extraData;
}
