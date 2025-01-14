// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import "../libraries/BombettaTypes.sol";

interface IBombetta {
    /// @notice Place a bid for a signed proof request.
    /// @param request The proof request that is being bid upon.
    /// @param signature The signature of proof request.
    /// @return reward The token reward available upon fulfillment.
    ///         provingDeadline The timestamp defining when the proof request must be resolved.
    function bid(ProofRequest calldata request, bytes calldata signature)
        external
        payable
        returns (uint256 reward, uint256 provingDeadline);

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
        returns (bool proverResolved);
}
