// SPDX-License-Identifier: MIT
pragma solidity ^0.8.23;

import "./interfaces/IBombetta.sol";
import "./libraries/BombettaTypes.sol";

/// @title  Bombetta
/// @author Taralli Labs
/// @notice a permission-less verifiable compute marketplace
abstract contract Bombetta is IBombetta {
    /// @notice emitted when a bid is placed successfully on a signed proof request.
    event Bid(address indexed signer, bytes32 indexed requestId, uint256 tokenReward, uint256 ethStake, address prover);
    /// @notice emitted when a signed proof request is successfully resolved.
    event Resolve(address indexed signer, bytes32 indexed requestId, address resolver);

    /// @notice Place a bid for a signed proof request.
    /// @param request The proof request that is being bid upon.
    /// @param signature The signature of proof request.
    /// @return reward The token reward available upon fulfillment.
    ///         provingDeadline The timestamp defining when the proof request must be resolved.
    function bid(ProofRequest calldata request, bytes calldata signature)
        external
        payable
        virtual
        returns (uint256 reward, uint256 provingDeadline)
    {}

    /// @notice Resolve a bid for a signed proof request.
    /// @param requestId The requestId associated to the ProofRequest being resolved.
    /// @param opaqueSubmission The opaque data that will be decoded by the market contract and passed to the verifier.
    ///                         Empty if the deadline has been reached and the prover is being slashed.
    /// @param partialCommitment The partial commitment to a field contained in opaqueSubmission needed to reconstruct
    ///                          that same final hash field within opaqueSubmission using another partial commitment
    ///                          value supplied by the proof requester before hand. (not always needed)
    /// @return proverResolved Value returning true if the original prover address that bid() to fulfill the request
    ///                        resolved the proof request. False if resolved by another address.
    function resolve(bytes32 requestId, bytes calldata opaqueSubmission, bytes32 partialCommitment)
        external
        virtual
        returns (bool proverResolved)
    {}
}
