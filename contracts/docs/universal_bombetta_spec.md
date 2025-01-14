# Universal Bombetta Specification

Summary:

This document details the implementation/specification of the universal bombetta market which can handle proof requests
from any proving system and circuit that can be verified in an evm execution environment (e.g. Ethereum). For generic 
bombetta details reference [bombetta_spec](./bombetta_spec.md).

## Types

```solidity
/// @notice Represents active job data of a `ProofRequest` that has been bid upon for resolution.
/// @custom:field requester address of the requester.
/// @custom:field prover address of the prove that bid on the proof request.
/// @custom:field resolutionDeadline timestamp by which the request must be resolved by the prover to avoid
///               slashing.
/// @custom:field token address of the erc20 token that will be given as a reward to the prover upon successful
///               resolution.
/// @custom:field requestReward amount of the reward token that will be given upon resolution 
///               of the proof request.
/// @custom:field proverStake amount of eth that is staked as collateral for the proof request.
/// @custom:field publicInputsCommitment hash of public inputs the requester committed to in their 
///               proof request.
/// @custom:field verifierDetails data specific to verification.
struct ActiveJob {
    address requester;
    address prover;
    uint256 resolutionDeadline;
    address token;
    uint256 requestReward;
    uint256 proverStake;
    bytes32 publicInputsCommitment;
    bytes verifierDetails;
}
    
    struct VerifierDetails {
        address verifier; // address of the verifier contract required by the requester
        bytes4 selector; // fn selector of the verifying function required by the requester
        // 
        
        uint256 publicInputsOffset; // offset of public inputs field within the proof submission data (opaqueSubmission)
        uint256 publicInputsLength; // length of public inputs field within the proof submission data (opaqueSubmission)
        // bool representing if a proof request requires a partial commitment result check in order to be resolved
        
        
        // predetermined partial commitment to the submitted final commitment result of this proof request's proof submission
        // data. The proof requester commits to this hash within their signature which is used to check equivalency when
        // recomputing the partial commitment result that is contained inside the proof submission data (opaqueSubmission)
        
    }

/// @notice Represents details for verification of a given proof request.
/// @custom:field verifier address of verifier contract.
/// @custom:field selector function selector of verifying function in the verifier contract.
/// @custom:field isShaCommitment boolean to chose between keccak256 or sha256 for checking commitments 
///               true = sha256, false = keccak256
/// @custom:field publicInputsOffset offset of public inputs field within the proof submission
///               data.
/// @custom:field publicInputsLength length of public inputs field.
/// @custom:field hasPartialCommitmentResultCheck boolean to chose whether or not to perform a partial commitment
///               result check.
/// @custom:field submittedPartialCommitmentResultOffset offset of the partial commitment final result field within the 
///               proof submission data (opaqueSubmission) that will be used to compare with the hash produced by ...
///               keccak256(predeterminedPartialCommitment + submittedPartialCommitment)
/// @custom:field submittedPartialCommitmentResultLength length of the partial commitment final result field.
/// @custom:field predeterminedPartialCommitment commitment hash that makes up the requester's portion of the final 
///               partial commitment result field which is hashed with the proof provider's portion to produce the hash
///               that will be compared to the submitted partial commitment result during resolution.
struct VerifierDetails {
    address verifier;
    bytes4 selector;
    bool isShaCommitment;
    uint256 publicInputsOffset;
    uint256 publicInputsLength;
    bool hasPartialCommitmentResultCheck;
    uint256 submittedPartialCommitmentResultOffset;
    uint256 submittedPartialCommitmentResultLength;
    bytes32 predeterminedPartialCommitment;
}
```

These data structures are specific to the universal bombetta and describe what data must be stored in order to resolve 
proof requests that commit to this bombetta market. The main thing of note is the verification logic, as there is no 
specific function signature or verifier contract enshrined in the contract which means the requester must commit to all
the specifics the prover must adhere to when resolving a proof request. Namely, the verifier details, which allow the 
market to verify the submitted public inputs/commitments (if needed) match the ones commited to by the requester and 
then make the call to the verification contract's verifying function using the opaqueSubmission data to assert proof 
validity before the eth and token transfer logic is determined (reward or slash).

## Constants & Storage

```solidity
// Canonical Permit2 contract
ISignatureTransfer public immutable PERMIT2;

// permit2 permitWitnessTransferFrom() type hashes
bytes32 public constant PERMIT_TRANSFER_FROM_TYPEHASH = keccak256(
"PermitTransferFrom(TokenPermissions permitted,address spender,uint256 nonce,uint256 deadline)TokenPermissions(address token,uint256 amount)"
);
bytes32 public constant TOKEN_PERMISSIONS_TYPEHASH = keccak256("TokenPermissions(address token,uint256 amount)");
string public constant PERMIT_TRANSFER_FROM_WITNESS_TYPEHASH_STUB =
"PermitWitnessTransferFrom(TokenPermissions permitted,address spender,uint256 nonce,uint256 deadline,";

// mapping to active job data
// requestId -> ActiveJob
mapping(bytes32 => ActiveJob) public activeJobData;
```

The universal bombetta market uses permit2 signatures that include a full witness that commits to the ProofRequest
itself...these fields below

```solidity
struct ProofRequest {
    // general
    address market;
    uint256 nonce;
    address token;
    // reward
    uint256 maxRewardAmount;
    uint256 minRewardAmount;
    // stake requirements
    uint128 minimumStake;
    // time constraints
    uint64 startTimestamp;
    uint64 deadline;
    uint32 provingTime;
    // verification commitments
    bytes32 publicInputsDigest;
    bytes extraData;
}
```

## Permit Signature

The implementation detail for how the witness is computed and the permitWitnessTransferFrom call is made is the 
following...

```solidity
//// BombettaProofrequestWitnessLib.sol
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.23;

import "./BombettaTypes.sol";

library BombettaProofRequestWitnessLib {
    string public constant FULL_PROOF_REQUEST_WITNESS_TYPE_STRING_STUB =
    "ProofRequest witness)TokenPermissions(address token,uint256 amount)ProofRequest(address market,uint256 nonce,address token,uint256 maxRewardAmount,uint256 minRewardAmount,uint128 minimumStake,uint64 startTimestamp,uint256 deadline,uint32 provingTime,bytes32 publicInputsDigest,bytes extraData";

    bytes internal constant PROOF_REQUEST_WITNESS_TYPE =
    "ProofRequest(address market,uint256 nonce,address token,uint256 maxRewardAmount,uint256 minRewardAmount,uint128 minimumStake,uint64 startTimestamp,uint256 deadline,uint32 provingTime,bytes32 publicInputsDigest,bytes extraData";

    bytes32 internal constant PROOF_REQUEST_WITNESS_TYPE_HASH = keccak256(PROOF_REQUEST_WITNESS_TYPE);

    /// @notice hash a ProofRequest, used as witness to the signature
    /// @param proofRequestWitness The ProofRequest object to hash
    function hash(ProofRequest memory proofRequestWitness) internal pure returns (bytes32) {
        return keccak256(
            abi.encode(
                PROOF_REQUEST_WITNESS_TYPE_HASH,
                proofRequestWitness.market,
                proofRequestWitness.nonce,
                proofRequestWitness.token,
                proofRequestWitness.maxRewardAmount,
                proofRequestWitness.minRewardAmount,
                proofRequestWitness.minimumStake,
                proofRequestWitness.startTimestamp,
                proofRequestWitness.deadline,
                proofRequestWitness.provingTime,
                proofRequestWitness.publicInputsDigest,
                proofRequestWitness.extraData
            )
        );
    }
}

//// Permit2's ISignatureTransfer.sol
/// @notice The signed permit message for a single token transfer
struct PermitTransferFrom {
    TokenPermissions permitted;
    // a unique value for every token owner's signature to prevent signature replays
    uint256 nonce;
    // deadline on the permit signature
    uint256 deadline;
}

//// UniversalBombetta.sol
/// @dev computes the digest the signer signed in order to perform the ec recover operation.
function computeDigest(ISignatureTransfer.PermitTransferFrom memory permit, bytes32 witness) public view returns (bytes32) {
    bytes32 typeHash = keccak256(
        abi.encodePacked(
            PERMIT_TRANSFER_FROM_WITNESS_TYPEHASH_STUB, RWL.FULL_PROOF_REQUEST_WITNESS_TYPE_STRING_STUB
        )
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
```

The witness data is added into the witness field of the permitWitnessTransferFrom() call based on the Permit 2 
specification. This means the permit transfer signature which contains commitments to all the witness data is now only
able to be used to make the token transfer to the contract for a potential reward if the input data of the msg.sender
calling the bid function is valid relative to the witness hash expected for a given signer.

## Bid Function

High level logic

1. check the submitted proof request is valid before verifying signature (e.g. timestamps are correct, request does not have an existing bid).
2. recover the signer of the submitted signature and then build the permitWitnessTransferFrom() call inputs including the witness hash.
3. calculate the token reward afforded to the bidder based on the timestamp the bid was submitted at.
4. transfer the erc20 token reward to the market contract using permit2 along with the eth sent in by the caller of bid()
5. store active job data within the activeJobData mapping for resolution of this proof request.

## Resolve Function

High level logic

1. check the submitted proof request has an active job associated to it so it is confirmed to have been bid upon before being resolved.
2. check timestamp of submission relative to active job deadline (if the timestamp is before the proof submission deadline, check the submitted proof, if not then slash the prover who bid on the proof request giving the tokens & eth to the requester).
3. check the proof submitted, if valid then the prover who bid upon the request is rewarded the erc20 tokens and receives their eth stake back, if it is invalid they get slashed and the tokens & eth are sent back to the requester.

## Checking Proof Submission

here is the implementation detail of how the universal bombetta checks the validity of submitted proofs in relation to a
proof request.

```solidity
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
        // Performs the call to the verifier contract and function selector that was committed to.
        return _callVerifier(vd.verifier, vd.selector, opaqueSubmission);
    }
```

The universal bombetta utilizes extra data in order to allow the proof requester to commit to the way in which the want
their proof request to be verified. This means commiting to the digest/hash of the public inputs/other commitments 
(if needed) to the proof they want computed, the address of the verifier contract where they want it to be verified, the
function selector of the function that must be called to verify the opaqueSubmission is correct. With this information
in the bombetta contract's storage upon calling resolve for a specific request, the proof requester can ensure the prover
has to verify any submission they make for a proof request under the rules defined by the witness data of their 
signature/intent. This design also allows for the property of being able to support & verify any proof from a request 
that has an implemented evm based verifier/verification function.
