// SPDX-License-Identifier: MIT
pragma solidity ^0.8.23;

/// @notice Represents an opaque request for a proof to be generated.
/// @custom:field signer The signer of the signature.
/// @custom:field market The address of the market that the proof request is being submitted to.
/// @custom:field nonce The permit signature nonce.
/// @custom:field rewardToken The address of the ERC20 token that the request creator/signer will reward the
///               proof provider with for resolving the request.
/// @custom:field maxRewardAmount The maximum amount of `token` that the request creator will pay the prover
///                               for a successful bid.
/// @custom:field minRewardAmount The minimum token amount available as a reward for bidding on the proof
///                               request. The reward will typically increase overtime until the full amount
///                               at the deadline of the signature is reached.
/// @custom:field minimumStake The minimum amount of stake that the prover is required to provide when bidding
///                            on a request denominated in wei. This value is slashed if the proof request is
///                            not resolved before the resolution deadline.
/// @custom:field startAuctionTimestamp The starting timestamp of the proof request auction.
/// @custom:field endAuctionTimestamp The deadline timestamp of the proof request auction & signature. If no
///                                   bid is submitted before this timestamp, the request is revoked.
/// @custom:field provingTime Amount of time (in seconds) to generate the proof. Time period starts when proof
///                           request auction is completed during bombetta.bid()'s execution.
/// @custom:field inputsCommitment Hash of all information the proof requester commits to in order to check
///                                that the opaque proof submission data at resolve() time is exactly what is
///                                requested. (for example the public inputs of the requested proof)
/// @custom:field extraData Opaque data relating to the bombetta market specific verification logic, if needed
///                         (verifier addr, fn selector, public inputs calldata location in opaqueSubmission, etc.)
struct ProofRequest {
    // general
    address signer;
    address market;
    uint256 nonce;
    // reward
    address rewardToken;
    uint256 maxRewardAmount;
    uint256 minRewardAmount;
    // stake requirements
    uint128 minimumStake;
    // time constraints
    uint64 startAuctionTimestamp;
    uint64 endAuctionTimestamp;
    uint32 provingTime;
    // verification commitments
    bytes32 inputsCommitment;
    bytes extraData;
}
