// SPDX-License-Identifier: MIT
pragma solidity ^0.8.23;

import "solady/utils/ECDSA.sol";
import {SafeTransferLib as STL} from "solady/utils/SafeTransferLib.sol";

import "./Bombetta.sol";
import "src/interfaces/IPermit2.sol";
import "./libraries/BombettaTypes.sol";
import "./libraries/BombettaErrors.sol";

/// @title  UniversalBombetta, a permission-less verifiable compute marketplace
/// @author Taralli Labs
contract UniversalBombetta is Bombetta {
    using ECDSA for bytes32;

    //////////////////////////////// TYPES ///////////////////////////////////

    struct ActiveJob {
        address requester; // address of the requester requesting the proof
        address prover; // address of the prover obligated to fufill the request
        uint256 resolutionDeadline; // deadline timestamp the proof provider must resolve the request by.
        address token; // reward token
        uint256 requestReward; // request reward token amount
        uint256 proverStake; // eth amount the prover staked
        bytes32 publicInputsCommitment; // hash of all knowledge the proof requester commits to
        bytes verifierDetails; // data specific to verification
    }

    struct VerifierDetails {
        address verifier; // address of the verifier contract required by the requester
        bytes4 selector; // fn selector of the verifying function required by the requester
        // bool to chose between keccak256 or sha256 for commitments, true = sha256, false = keccak256
        bool isShaCommitment;
        uint256 publicInputsOffset; // offset of public inputs field within the proof submission data (opaqueSubmission)
        uint256 publicInputsLength; // length of public inputs field within the proof submission data (opaqueSubmission)
        // bool representing if a proof request requires a partial commitment result check in order to be resolved
        bool hasPartialCommitmentResultCheck;
        // offset & length of the partial commitment result field within the proof submission data (opaqueSubmission)
        // that will be used to compare with the hash produced by ...
        // keccak256(predeterminedPartialCommitment + submittedPartialCommitment)
        uint256 submittedPartialCommitmentResultOffset;
        uint256 submittedPartialCommitmentResultLength;
        // predetermined partial commitment to the submitted final commitment result of this proof request's proof submission
        // data. The proof requester commits to this hash within their signature which is used to check equivalency when
        // recomputing the partial commitment result that is contained inside the proof submission data (opaqueSubmission)
        bytes32 predeterminedPartialCommitment;
    }

    ////////////////////////////// CONSTANTS /////////////////////////////////

    // Canonical Permit2 contract
    IPermit2 public immutable PERMIT2;
    // permit2 permitWitnessTransferFrom() type hashes
    bytes32 public constant PERMIT_TRANSFER_FROM_TYPEHASH = keccak256(
        "PermitTransferFrom(TokenPermissions permitted,address spender,uint256 nonce,uint256 deadline)TokenPermissions(address token,uint256 amount)"
    );
    bytes32 public constant TOKEN_PERMISSIONS_TYPEHASH = keccak256("TokenPermissions(address token,uint256 amount)");
    string public constant PERMIT_TRANSFER_FROM_WITNESS_TYPEHASH_STUB =
        "PermitWitnessTransferFrom(TokenPermissions permitted,address spender,uint256 nonce,uint256 deadline,";
    string public constant FULL_PROOF_REQUEST_WITNESS_TYPE_STRING_STUB =
        "ProofRequest witness)TokenPermissions(address token,uint256 amount)ProofRequest(address signer,address market,uint256 nonce,address token,uint256 maxRewardAmount,uint256 minRewardAmount,uint128 minimumStake,uint64 startAuctionTimestamp,uint64 endAuctionTimestamp,uint32 provingTime,bytes32 publicInputsCommitment,bytes extraData)";
    bytes public constant PROOF_REQUEST_WITNESS_TYPE =
        "ProofRequest(address signer,address market,uint256 nonce,address token,uint256 maxRewardAmount,uint256 minRewardAmount,uint128 minimumStake,uint64 startAuctionTimestamp,uint64 endAuctionTimestamp,uint32 provingTime,bytes32 publicInputsCommitment,bytes extraData)";
    bytes32 public constant PROOF_REQUEST_WITNESS_TYPE_HASH = keccak256(PROOF_REQUEST_WITNESS_TYPE);

    /////////////////////////////// STORAGE //////////////////////////////////

    /// @notice mapping to active job data
    // requestId -> ActiveJob
    mapping(bytes32 => ActiveJob) public activeJobData;

    //////////////////////////// CONSTRUCTOR /////////////////////////////////

    constructor(IPermit2 _permit) {
        PERMIT2 = _permit;
    }

    ///////////////////////// EXTERNAL FUNCTIONS /////////////////////////////

    /// @notice Place a bid for a signed proof request.
    /// @param request The proof request that is being bid upon.
    /// @param signature The signature of proof request.
    /// @return reward The token reward available upon fulfillment.
    ///         provingDeadline The timestamp defining when the proof request must be resolved.
    function bid(ProofRequest calldata request, bytes calldata signature)
        external
        payable
        override
        returns (uint256 reward, uint256 provingDeadline)
    {
        // check request data
        if (block.timestamp < request.startAuctionTimestamp) revert InvalidRequest();
        if (block.timestamp > request.endAuctionTimestamp) revert InvalidRequest();
        if (msg.value < request.minimumStake) revert InvalidRequest();
        // compute request ID
        bytes32 requestId = computeRequestId(request, signature);
        // assert that only 1 bid can be placed for a given request
        if (activeJobData[requestId].requester != address(0)) revert AuctionEnded();

        // Build permit struct
        ISignatureTransfer.PermitTransferFrom memory permit = ISignatureTransfer.PermitTransferFrom({
            permitted: ISignatureTransfer.TokenPermissions({token: request.token, amount: request.maxRewardAmount}),
            nonce: request.nonce,
            deadline: request.endAuctionTimestamp
        });

        // Generate witness hash
        bytes32 witness = computeWitnessHash(request);

        // Calculate the reward amount based on the auction parameters
        reward = calculateReward(
            request.startAuctionTimestamp, request.endAuctionTimestamp, request.minRewardAmount, request.maxRewardAmount
        );

        PERMIT2.permitWitnessTransferFrom(
            permit,
            ISignatureTransfer.SignatureTransferDetails({to: address(this), requestedAmount: reward}),
            request.signer,
            witness,
            FULL_PROOF_REQUEST_WITNESS_TYPE_STRING_STUB,
            signature
        );

        // store bid data for resolution
        provingDeadline = block.timestamp + request.provingTime;
        activeJobData[requestId] = ActiveJob({
            requester: request.signer,
            prover: msg.sender,
            resolutionDeadline: provingDeadline,
            token: request.token,
            proverStake: msg.value,
            requestReward: reward,
            publicInputsCommitment: request.publicInputsCommitment,
            verifierDetails: request.extraData
        });

        emit Bid(request.signer, requestId, reward, request.minimumStake, msg.sender);
    }

    /// @notice Resolve a bid for a signed proof request.
    /// @param requestId The requestId associated to the ProofRequest being resolved.
    /// @param opaqueSubmission The opaque data that will be decoded by the market contract and passed to the verifier.
    ///                         Empty if the deadline has been reached and the prover is being slashed.
    /// @return proverResolved Value returning true if the original prover address that bid() to fulfill the request
    ///                        resolved the proof request. Returning false if the provider failed to fulfill the request 
    ///                        and the stale request is resolved by another address.
    function resolve(bytes32 requestId, bytes calldata opaqueSubmission, bytes32 submittedPartialCommitment)
        external
        override
        returns (bool proverResolved)
    {
        // load active job data
        ActiveJob memory activeJob = activeJobData[requestId];
        address token = activeJob.token;
        // if the request has a valid deadline, check proof validity
        if (block.timestamp <= activeJob.resolutionDeadline) {
            if (msg.sender != activeJob.prover) {
                // alternative proof provider is not allowed until resolutionDeadline has passed
                revert InvalidResolver();
            }
            // check proof is valid
            proverResolved = _checkProofSubmission(
                activeJob.publicInputsCommitment,
                activeJob.verifierDetails,
                opaqueSubmission,
                submittedPartialCommitment
            );
            // if proof is valid then reward prover
            if (proverResolved) {
                // reward prover
                address prover = activeJob.prover;
                payable(prover).transfer(activeJob.proverStake);
                STL.safeTransfer(token, prover, activeJob.requestReward);
            } else {
                // invalid proof, slash prover
                address requester = activeJob.requester;
                payable(requester).transfer(activeJob.proverStake);
                STL.safeTransfer(token, requester, activeJob.requestReward);
            }
        } else {
            // proof request expired, slash prover
            address requester = activeJob.requester;
            payable(requester).transfer(activeJob.proverStake);
            STL.safeTransfer(token, requester, activeJob.requestReward);
        }
        emit Resolve(activeJob.requester, requestId, msg.sender);
    }

    ///////////////////////// INTERNAL FUNCTIONS /////////////////////////////

    /// @dev Function to calculate the reward amount based on the auction parameters with linear increase from a minimum price
    function calculateReward(uint256 startTimestamp, uint256 endTimestamp, uint256 minReward, uint256 totalAmount)
        internal
        view
        returns (uint256)
    {
        if (startTimestamp >= endTimestamp) revert InvalidTimestamp();
        if (block.timestamp < startTimestamp) revert InvalidTimestamp();
        if (block.timestamp >= endTimestamp) return totalAmount; // reached deadline, max reward

        uint256 elapsedTime = block.timestamp - startTimestamp;
        uint256 totalDuration = endTimestamp - startTimestamp;

        // Calculate the reward increase factor
        uint256 increaseFactor = (elapsedTime * 1e18) / totalDuration;
        // Calculate the increased amount
        uint256 increaseAmount = (increaseFactor * (totalAmount - minReward)) / 1e18;
        // Calculate the current reward
        return minReward + increaseAmount;
    }

    /// @dev check the correctness of the submitted proof during execution of resolve()
    function _checkProofSubmission(
        bytes32 publicInputsCommitment,
        bytes memory verifierDetails,
        bytes calldata opaqueSubmission,
        bytes32 partialCommitment
    ) internal returns (bool) {
        VerifierDetails memory vd = abi.decode(verifierDetails, (VerifierDetails));

        // check public inputs if needed
        if (vd.publicInputsLength > 0) {
            // check partial commitment + outside hash from resolver matches final result found in opaqueSubmission data if needed
            if (vd.hasPartialCommitmentResultCheck) {
                // extract the submitted final result field from opaqueSubmission data
                if (
                    vd.submittedPartialCommitmentResultOffset + vd.submittedPartialCommitmentResultLength
                        > opaqueSubmission.length
                ) revert InvalidExpectedPartialCommitmentResultField();
                bytes memory finalCommitmentResult = opaqueSubmission[
                    vd.submittedPartialCommitmentResultOffset:
                        vd.submittedPartialCommitmentResultOffset + vd.submittedPartialCommitmentResultLength
                ];

                // check that the submitted final commitment result is equal to the computed one from the proof requester
                // provided `predeterminedPartialCommitment` + proof provider's
                if (vd.isShaCommitment) {
                    if (
                        sha256(abi.encodePacked(finalCommitmentResult))
                            != sha256(abi.encodePacked(vd.predeterminedPartialCommitment, partialCommitment))
                    ) {
                        return false;
                    }
                } else {
                    if (
                        keccak256(abi.encodePacked(finalCommitmentResult))
                            != keccak256(abi.encodePacked(vd.predeterminedPartialCommitment, partialCommitment))
                    ) {
                        return false;
                    }
                }
            }

            // extract submitted public inputs field within opaqueSubmission data
            if (vd.publicInputsOffset + vd.publicInputsLength > opaqueSubmission.length) {
                revert InvalidPublicInputsCommitmentField();
            }
            bytes memory submittedPublicInputs =
                opaqueSubmission[vd.publicInputsOffset:vd.publicInputsOffset + vd.publicInputsLength];

            // Check submitted vs expected public input(s) commitments
            if (vd.isShaCommitment) {
                if (publicInputsCommitment != sha256(submittedPublicInputs)) {
                    return false;
                }
            } else {
                if (publicInputsCommitment != keccak256(submittedPublicInputs)) {
                    return false;
                }
            }
        }
        return _callVerifier(vd.verifier, vd.selector, opaqueSubmission);
    }

    /// @dev Perform the static call to the verifier and return the result of the call
    function _callVerifier(address verifier, bytes4 selector, bytes calldata opaqueSubmission)
        internal
        returns (bool)
    {
        bool success;
        assembly {
            let ptr := mload(0x40)
            // Store the function selector
            mstore(ptr, selector)
            // Copy the opaqueSubmission data right after the function selector
            calldatacopy(add(ptr, 4), opaqueSubmission.offset, opaqueSubmission.length)
            let returndata_size := 32
            let returndata := add(ptr, add(opaqueSubmission.length, 4))
            success := call(gas(), verifier, 0, ptr, add(opaqueSubmission.length, 4), returndata, returndata_size)
        }
        return success;
    }

    /// @dev hashes the proof request and signature for use as the request ID in mapping `activeJobData`
    /// request ID = keccak256(request + signature)
    function computeRequestId(ProofRequest calldata request, bytes calldata signature) public pure returns (bytes32) {
        return keccak256(
            abi.encode(
                request.market,
                request.nonce,
                request.token,
                request.maxRewardAmount,
                request.minRewardAmount,
                request.minimumStake,
                request.startAuctionTimestamp,
                request.endAuctionTimestamp,
                request.provingTime,
                request.publicInputsCommitment,
                keccak256(abi.encode(request.extraData)),
                keccak256(abi.encode(signature))
            )
        );
    }

    /// @notice hash a ProofRequest, used as witness to the signature
    /// @param proofRequestWitness The ProofRequest object to hash
    function computeWitnessHash(ProofRequest memory proofRequestWitness) public pure returns (bytes32) {
        return keccak256(
            abi.encode(
                PROOF_REQUEST_WITNESS_TYPE_HASH,
                proofRequestWitness.signer,
                proofRequestWitness.market,
                proofRequestWitness.nonce,
                proofRequestWitness.token,
                proofRequestWitness.maxRewardAmount,
                proofRequestWitness.minRewardAmount,
                proofRequestWitness.minimumStake,
                proofRequestWitness.startAuctionTimestamp,
                proofRequestWitness.endAuctionTimestamp,
                proofRequestWitness.provingTime,
                proofRequestWitness.publicInputsCommitment,
                keccak256(proofRequestWitness.extraData)
            )
        );
    }

    /// @dev computes the digest the signer signed in order to perform the ec recover operation.
    function computePermitDigest(ISignatureTransfer.PermitTransferFrom memory permit, bytes32 witness)
        public
        view
        returns (bytes32)
    {
        bytes32 typeHash = keccak256(
            abi.encodePacked(PERMIT_TRANSFER_FROM_WITNESS_TYPEHASH_STUB, FULL_PROOF_REQUEST_WITNESS_TYPE_STRING_STUB)
        );

        bytes32 tokenPermissionsHash = keccak256(abi.encode(TOKEN_PERMISSIONS_TYPEHASH, permit.permitted));

        bytes32 dataHash =
            keccak256(abi.encode(typeHash, tokenPermissionsHash, address(this), permit.nonce, permit.deadline, witness));

        return _hashTypedData(PERMIT2.DOMAIN_SEPARATOR(), dataHash);
    }

    /// @notice Creates an EIP-712 typed data hash
    function _hashTypedData(bytes32 domainSeparator, bytes32 dataHash) internal pure returns (bytes32) {
        return keccak256(abi.encodePacked("\x19\x01", domainSeparator, dataHash));
    }
}
