# Universal Porchetta Specification

Summary:

This document details the implementation/specification of the universal porchetta market which is an implementation of the porchetta standard
that can handle offers from any proving system/proof type and circuit that can be verified in an evm execution environment (e.g. Ethereum).
For generic porchetta details reference [porchetta_spec](./porchetta_spec.md).

## Types

```solidity
struct ActiveProofOffer {
    // address of the proof provider obligated to resolve the proof offer.
    address provider;
    // address of the requester that bid on the proof offer.
    address requester;
    // deadline timestamp the proof provider must resolve the offer by.
    uint256 resolutionDeadline;
    // reward token.
    address rewardToken;
    // offer reward token amount.
    uint256 rewardAmount;
    // stake token used as collateral for the proof offer.
    address stakeToken;
    // amount of `stakeToken` proof provider has staked.
    uint256 stakeAmount;
    // hash of all knowledge the proof provider commits to within the opaque submission excluding the proof itself.
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
}
```

These data structures are specific to the universal porchetta and describe what data must be stored in order to resolve
offers that commit to this porchetta market. The main thing of note is the verification logic, as there is no 
specific function signature or verifier contract enshrined in the contract which means the provider must commit to all
the specifics of the proof they are providing such that requester's bidding on the offers can rely on the resolution process. 
Namely, the verifier details, which allow the market contract to verify the submitted inputs/commitments (if needed) match the ones commited 
to by the provider originally in the ProofOffer when making the final call to the verification contract's verifying function. This is done
by using the opaqueSubmission data to assert proof validity before the stake and reward token transfer logic is determined (reward or slash).

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
string public constant FULL_PROOF_OFFER_WITNESS_TYPE_STRING_STUB =
    "ProofOffer witness)TokenPermissions(address token,uint256 amount)ProofOffer(address signer,address market,uint256 nonce,address rewardToken,uint256 rewardAmount,address stakeToken,uint256 stakeAmount,uint64 startAuctionTimestamp,uint64 endAuctionTimestamp,uint32 provingTime,bytes32 inputsCommitment,bytes extraData)";
bytes public constant PROOF_OFFER_WITNESS_TYPE =
    "ProofOffer(address signer,address market,uint256 nonce,address rewardToken,uint256 rewardAmount,address stakeToken,uint256 stakeAmount,uint64 startAuctionTimestamp,uint64 endAuctionTimestamp,uint32 provingTime,bytes32 inputsCommitment,bytes extraData)";
bytes32 public constant PROOF_OFFER_WITNESS_TYPE_HASH = keccak256(PROOF_OFFER_WITNESS_TYPE);

// mapping to active proof offer data
// offerId -> ActiveProofOffer
mapping(bytes32 => ActiveProofOffer) public activeProofOfferData;
```

The universal porchetta market uses permit2 signatures that include a full witness that commits to the Bombetta.ProofOffer
itself...

## Permit Signature

The implementation detail for how the witness is computed and the permitWitnessTransferFrom call is made is the 
following...

```solidity
/// @notice hash a ProofRequest, used as witness to the signature
/// @param proofOfferWitness The ProofOffer object to hash
function computeWitnessHash(ProofOffer memory proofOfferWitness) public pure returns (bytes32) {
    return keccak256(
        abi.encode(
            PROOF_OFFER_WITNESS_TYPE_HASH,
            proofOfferWitness.signer,
            proofOfferWitness.market,
            proofOfferWitness.nonce,
            proofOfferWitness.rewardToken,
            proofOfferWitness.rewardAmount,
            proofOfferWitness.stakeToken,
            proofOfferWitness.stakeAmount,
            proofOfferWitness.startAuctionTimestamp,
            proofOfferWitness.endAuctionTimestamp,
            proofOfferWitness.provingTime,
            proofOfferWitness.inputsCommitment,
            keccak256(proofOfferWitness.extraData)
        )
    );
}

/// @dev computes the digest the signer signed in order to perform the ec recover operation.
function computePermitDigest(ISignatureTransfer.PermitTransferFrom memory permit, bytes32 witness)
    external
    view
    returns (bytes32)
{
    bytes32 typeHash = keccak256(
        abi.encodePacked(PERMIT_TRANSFER_FROM_WITNESS_TYPEHASH_STUB, FULL_PROOF_OFFER_WITNESS_TYPE_STRING_STUB)
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
able to be used to make the token transfer to the contract for a potential active offer if the input data of the msg.sender
(requester) calling the bid function is valid relative to the witness hash expected for a given signer (provider).

## Provider IDs

Once a proof offer has been bid upon a unique ID is generated for it and stored for use later during the resolution phase. The ID is
generated using the following logic...

```solidity
/// @dev hashes the proof offer for use as the offer ID in mapping `activeProofOfferData`
function computeOfferId(ProofOffer calldata offer) public pure returns (bytes32) {
    return keccak256(
        abi.encode(
            offer.signer,
            offer.market,
            offer.nonce,
            offer.rewardToken,
            offer.rewardAmount,
            offer.stakeToken,
            offer.stakeAmount,
            offer.startAuctionTimestamp,
            offer.endAuctionTimestamp,
            offer.provingTime,
            offer.inputsCommitment,
            offer.extraData
        )
    );
}
```

## Bid Function

High level logic

1. check the submitted offer is valid before permit2 call (e.g. timestamps are correct, offer does not have an existing bid).
2. validate signature & transfer the erc20 token stake to the market contract using permit2, along with the token reward sent in by the caller of bid()
5. store active proof offer data within the activeProofOffer Data mapping for future resolution of this offer.

## Resolve Function

High level logic

1. check the submitted offer has an active offer ID associated to it so it is confirmed to have been bid upon before being resolved.
2. check the timestamp of proof submission relative to active proof offer resolution deadline (if the timestamp is before the proof submission deadline, check the submitted proof, if not then slash the prover who bid on the offer giving the stake/reward tokens to the requester).
3. check the submitted proof, if valid then the requester who bid upon the offer rewards the provider and the provider receives their token stake back, if it is invalid the provider get slashed and the stake/reward tokens are sent to the requester.

## Checking Proof Submission

Here is the implementation detail of how the universal porchetta contract checks the validity of submitted proofs in relation to an offer.
This function is designed to universally check across many combinations of onchain "verifier" APIs wether it be Groth, Plonk, other zk system
or simply a call to perform a merkle inclusion proof on an external contract. It is up to the provider making the signature what methodology
they choose to suffciently verify the proof offer they made such that a requester will bid on their offer. Hence the name Universal Porchetta.

```solidity
/// @dev check the correctness of the submitted proof during execution of resolve()
function _checkProofSubmission(
    bytes32 inputsCommitment,
    bytes memory verifierDetails,
    bytes calldata opaqueSubmission
) internal returns (bool) {
    VerifierDetails memory vd = abi.decode(verifierDetails, (VerifierDetails));
    // check inputs if needed
    if (vd.inputsLength > 0) {
        // extract submitted inputs field within opaqueSubmission data
        if (vd.inputsOffset + vd.inputsLength > opaqueSubmission.length) {
            revert InvalidInputsCommitmentField();
        }
        bytes memory submittedInputs = opaqueSubmission[vd.inputsOffset:vd.inputsOffset + vd.inputsLength];
        // Check submitted vs expected input(s) commitments
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

/// @dev Perform the call to the verifier and return the result of the call
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
