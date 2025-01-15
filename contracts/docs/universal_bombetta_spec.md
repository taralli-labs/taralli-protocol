# Universal Bombetta Specification

Summary:

This document details the implementation/specification of the universal bombetta market which is an implementation of the bombetta standard
that can handle requests from any proving system/proof type and circuit that can be verified in an evm execution environment (e.g. Ethereum).
For generic bombetta details reference [bombetta_spec](./bombetta_spec.md).

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
requests that commit to this bombetta market. The main thing of note is the verification logic, as there is no 
specific function signature or verifier contract enshrined in the contract which means the requester must commit to all
the specifics the provider must adhere to when submitting a proof in the resolution process. Namely, the verifier details, 
which allow the market to verify the submitted public inputs/commitments (if needed) match the ones commited to by the requester 
and then make the call to the verification contract's verifying function using the opaqueSubmission data to assert proof 
validity before the eth and token transfer logic is determined (reward or slash).

## Constants & Storage

```solidity


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

// mapping to active job data
// requestId -> ActiveJob
mapping(bytes32 => ActiveJob) public activeJobData;
```

The universal bombetta market uses permit2 signatures that include a full witness that commits to the ProofRequest
itself...these fields below

```solidity
struct ProofRequest {
    // general
    address signer;
    address market;
    uint256 nonce;
    address token;
    // reward
    uint256 maxRewardAmount;
    uint256 minRewardAmount;
    // stake requirements
    uint128 minimumStake;
    // time constraints
    uint64 startAuctionTimestamp;
    uint64 endAuctionTimestamp;
    uint32 provingTime;
    // verification commitments
    bytes32 publicInputsCommitment;
    bytes extraData;
}
```

## Permit Signature

The implementation detail for how the witness is computed and the permitWitnessTransferFrom call is made is the 
following...

```solidity
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

/// @notice permit2 trasnfer from call
PERMIT2.permitWitnessTransferFrom(
    permit,
    ISignatureTransfer.SignatureTransferDetails({to: address(this), requestedAmount: reward}),
    request.signer,
    witness,
    FULL_PROOF_REQUEST_WITNESS_TYPE_STRING_STUB,
    signature
);
```

The witness data is added into the witness field of the permitWitnessTransferFrom() call based on the Permit 2 
specification. This means the permit transfer signature which contains commitments to all the witness data is now only
able to be used to make the token transfer to the contract for a potential reward if the input data of the msg.sender
calling the bid function is valid relative to the witness hash expected for a given signer.

## Bid Function

High level logic

1. check the submitted request is valid before verifying signature (e.g. timestamps are correct, request does not have an existing bid).
2. ecrecover the signer of the submitted signature to validate the signature and then build the permitWitnessTransferFrom() call inputs including the witness hash.
3. calculate the token reward afforded to the bidder based on the timestamp the bid was submitted at.
4. transfer the erc20 token reward to the market contract using permit2 along with the eth stake sent in by the caller of bid()
5. store active job data within the activeJobData mapping for future resolution of this request.

## Resolve Function

High level logic

1. check the submitted request has an active job associated to it so it is confirmed to have been bid upon before being resolved.
2. check timestamp of submission relative to active job deadline (if the timestamp is before the proof submission deadline, check the submitted proof, if not then slash the prover who bid on the request giving the tokens & eth to the requester).
3. check the proof submitted, if valid then the provider who bid upon the request is rewarded the erc20 tokens and receives their eth stake back, if it is invalid they get slashed and the tokens & eth are sent to the requester.

## Checking Proof Submission

Here is the implementation detail of how the universal bombetta contract checks the validity of submitted proofs in relation to a request.
This function is designed to universally check across many combinations of onchain "verifier" APIs wether it be a Groth, Plonk, other zk system
or simply a call to perform a simple merkle inclusion proof on an external contract. It is up to the requester making the signature what methodology
they trust to suffciently verify that the computation request they made was in fact verified to be done correctly based on the outcome of the resolve 
function. Hence the name Universal Bombetta

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
    return _callVerifier(vd.verifier, vd.selector, opaqueSubmission);
}
```
