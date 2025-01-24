// SPDX-License-Identifier: MIT
pragma solidity ^0.8.23;

import "../interfaces/IPorchetta.sol";
import "../libraries/PorchettaTypes.sol";

/// @title  Porchetta
/// @author Taralli Labs
/// @notice a permission-less verifiable compute marketplace
abstract contract Porchetta is IPorchetta {
    /// @notice emitted when a bid is placed successfully on a signed proof offer.
    event Bid(
        address indexed signer,
        bytes32 indexed offerId,
        address rewardToken,
        uint256 rewardAmount,
        address stakeToken,
        uint256 stakeAmount,
        address requester
    );
    /// @notice emitted when a signed proof offer is successfully resolved.
    event Resolve(address indexed signer, bytes32 indexed offerId, address resolver);

    /// @notice Place a bid for a signed proof offer.
    /// @param offer The offer that is being bid upon.
    /// @param signature The signature of the proof offer.
    /// @param submittedPartialCommitment The partial commitment provided by the proof requester to be used in checking the
    ///                                   correctness of the submission alongside the requester's `predeterminedPartialCommitment`
    ///                                   if needed.
    /// @return rewardToken the address of the reward token
    ///         rewardAmount The token reward amount available upon resolution.
    ///         provingDeadline The timestamp defining when the proof offer must be resolved.
    function bid(ProofOffer calldata offer, bytes calldata signature, bytes32 submittedPartialCommitment)
        external
        virtual
        returns (address, uint256, uint256)
    {}

    /// @notice Resolve a bid for a signed proof offer..
    /// @param offerId The offerId associated to the ProofOffer being resolved
    /// @param opaqueSubmission The opaque data that will be decoded by the
    ///                         market contract and passed to the verifier.
    ///                         Empty if the deadline has been reached and
    ///                         the proof provider is being slashed.
    /// @return proverResolved Value returning true if the original prover address that bid() to fulfill the request
    ///                        resolved the proof offer. Returning false if the provider failed to fulfill the request
    ///                        and the stale request is resolved by another address.
    function resolve(bytes32 offerId, bytes calldata opaqueSubmission) external virtual returns (bool proverResolved) {}
}
