# Universal Bombetta Specification

Summary:

This document details the implementation/specification of the universal bombetta market which is an implementation of the bombetta standard
that can handle requests from any proving system/proof type and circuit that can be verified in an evm execution environment (e.g. Ethereum).
For generic bombetta details reference [bombetta_spec](./bombetta_spec.md).

## Types

```solidity
struct ActiveProofRequest {
    // address of the requester requesting the proof.
    address requester;
    // address of the proof provider obligated to fufill the request.
    address provider;
    // deadline timestamp the proof provider must resolve the request by.
    uint256 resolutionDeadline;
    // reward token.
    address rewardToken;
    // request reward token amount.
    uint256 rewardAmount;
    // eth amount the proof provider has staked.
    uint256 providerStake;
    // hash of all knowledge the proof requester commits to. (e.g. public inputs of requested proof)
    bytes32 inputsCommitment;
    // data specific to verification.
    bytes verifierDetails;
}

struct VerifierDetails {
    // address of the verifier contract required by the requester
    address verifier;
    // fn selector of the verifying function required by the requester
    bytes4 selector;
    // bool to chose between keccak256 or sha256 for commitments, true = sha256, false = keccak256
    bool isShaCommitment;
    // offset of inputs field within the proof submission data (opaqueSubmission)
    uint256 inputsOffset;
    // length of inputs field within the proof submission data (opaqueSubmission)
    uint256 inputsLength;
    // bool representing if a proof request requires a partial commitment result check in order to be resolved
    bool hasPartialCommitmentResultCheck;
    // offset & length of the partial commitment result field within the proof submission data (opaqueSubmission)
    // that will be used to compare with the hash produced by ...
    // keccak256(predeterminedPartialCommitment + submittedPartialCommitment)
    uint256 submittedPartialCommitmentResultOffset;
    uint256 submittedPartialCommitmentResultLength;
    // predetermined partial commitment to the submitted final commitment result of the proof submission data.
    // The proof requester commits to this hash within their signature which is used to check equivalency when
    // recomputing the partial commitment result that is contained inside the proof submission data (opaqueSubmission)
    bytes32 predeterminedPartialCommitment;
}
```

These data structures are specific to the universal bombetta and describe what data must be stored in order to resolve
requests that commit to this bombetta market. The main thing of note is the verification logic, as there is no 
specific function signature or verifier contract enshrined in the contract which means the requester must commit to all
the specifics the provider must adhere to when submitting a proof in the resolution process. Namely, the verifier details, 
which allow the market to verify the submitted inputs/commitments (if needed) match the ones commited to by the requester 
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

/// @notice mapping to active proof request data
// requestId -> ActiveProofRequest
mapping(bytes32 => ActiveProofRequest) public activeProofRequestData;
```

The universal bombetta market uses permit2 signatures that include a full witness that commits to the Bombetta.ProofRequest
itself...

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
            proofRequestWitness.rewardToken,
            proofRequestWitness.maxRewardAmount,
            proofRequestWitness.minRewardAmount,
            proofRequestWitness.minimumStake,
            proofRequestWitness.startAuctionTimestamp,
            proofRequestWitness.endAuctionTimestamp,
            proofRequestWitness.provingTime,
            proofRequestWitness.inputsCommitment,
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
```

The witness data is added into the witness field of the permitWitnessTransferFrom() call based on the Permit 2 
specification. This means the permit transfer signature which contains commitments to all the witness data is now only
able to be used to make the token transfer to the contract for a potential reward if the input data of the msg.sender
calling the bid function is valid relative to the witness hash expected for a given signer.

## Request IDs

Once a proof request has been bid upon a unique ID is generated for it and stored for use later during the resolution phase. The ID is
generated using the following logic...

```solidity
/// @dev hashes the proof request and signature for use as the request ID in mapping `activeJobData`
/// request ID = keccak256(request + signature)
function computeRequestId(ProofRequest calldata request, bytes calldata signature) public pure returns (bytes32) {
    return keccak256(
        abi.encode(
            request.market,
            request.nonce,
            request.rewardToken,
            request.maxRewardAmount,
            request.minRewardAmount,
            request.minimumStake,
            request.startAuctionTimestamp,
            request.endAuctionTimestamp,
            request.provingTime,
            request.inputsCommitment,
            keccak256(abi.encode(request.extraData)),
            keccak256(abi.encode(signature))
        )
    );
}
```

## Bid Function

High level logic

1. check the submitted request is valid before permit2 call (e.g. timestamps are correct, request does not have an existing bid).
2. calculate the token reward afforded to the bidder based on the timestamp the bid was submitted at.
3. validate signature & transfer the erc20 token reward to the market contract using permit2, along with the eth stake sent in by the caller of bid()
5. store active proof request data within the activeProofRequestData mapping for future resolution of this request.

## Resolve Function

High level logic

1. check the submitted request has an active request ID associated to it so it is confirmed to have been bid upon before being resolved.
2. check the timestamp of proof submission relative to active proof request resolution deadline (if the timestamp is before the proof submission deadline, check the submitted proof, if not then slash the prover who bid on the request giving the tokens & eth to the requester).
3. check the submitted proof, if valid then the provider who bid upon the request is rewarded the erc20 tokens and receives their eth stake back, if it is invalid they get slashed and the tokens & eth are sent to the requester.

## Checking Proof Submission

Here is the implementation detail of how the universal bombetta contract checks the validity of submitted proofs in relation to a request.
This function is designed to universally check across many combinations of onchain "verifier" APIs wether it be a Groth, Plonk, other zk system
or simply a call to perform a merkle inclusion proof on an external contract. It is up to the requester making the signature what methodology
they trust to suffciently verify that the proof request they made was in fact verified to be done correctly based on the outcome of the resolve 
function. Hence the name Universal Bombetta

```solidity
/// @dev check the correctness of the submitted proof during execution of resolve()
function _checkProofSubmission(
    bytes32 inputsCommitment,
    bytes memory verifierDetails,
    bytes calldata opaqueSubmission,
    bytes32 partialCommitment
) internal returns (bool) {
    VerifierDetails memory vd = abi.decode(verifierDetails, (VerifierDetails));
    // check inputs if needed
    if (vd.inputsLength > 0) {
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
        // extract submitted inputs field within opaqueSubmission data
        if (vd.inputsOffset + vd.inputsLength > opaqueSubmission.length) {
            revert InvalidInputsCommitmentField();
        }
        bytes memory submittedInputs = opaqueSubmission[vd.inputsOffset:vd.inputsOffset + vd.inputsLength];
        // Check submitted vs expected public input(s) commitments
        if (vd.isShaCommitment) {
            if (inputsCommitment != sha256(submittedInputs)) {
                return false;
            }
        } else {
            if (inputsCommitment != keccak256(submittedInputs)) {
                return false;
            }
        }
    }
    return _callVerifier(vd.verifier, vd.selector, opaqueSubmission);
}
```

The last step of resolution is to call the verifying function using the opaqueSubmission data as the call's calldata like below.
If th result from the call is a success then the submission is considered valid allowing the contract to reward the provider for
resolving the request correctly.

```solidity
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
```
