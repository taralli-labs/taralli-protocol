# DESIGN DOC: Aligned Layer Bombetta Market

## Summary

The goal is to build a working implementation of a bombetta/other proving market that utilizes an off-chain verification
layer (e.g. AlignedLayer) as means to resolve the proof request instead of verifying the proof directly on-chain which
is often costly for the proof provider fulfilling the intent.

## Problem Statement

Move proof verification for proving marketplaces off-chain in a trust minimized way to reduce the cost of using the 
on-chain market by a large margin and make solving intents cheaper for proof providers where trusting off-chain 
verification layers is suffcient.

## Proposed Solution

The `AlignedLayerBombetta` bombetta market contract is one of the proposed solutions for off-chain verification service
integration for on-chain proving markets. The bombetta contract uses the `AlignedLayerServiceManager` contract as the 
source of truth, which is the on-chain component of the aligned layer protocol.

The `AlignedLayerServiceManager` contract stores merkle roots which reference batches of proofs (AKA `batchMerkleRoot`)
that have been submitted to aligned layer's api and then verified off-chain using aligned layer consensus. These 
batchMerkleRoots are read by the `AlignedLayerBombetta` contract and allow the capability to perform inclusion proofs of
various proof submissions that took place off-chain within aligned layer using the on-chain state contained by the 
batchMerkleRoots within the execution of the bombetta market's resolve function. Implementation details are below.

## Actors

refer to bombetta specification, it is the same (e.g. proof requester & proof provider).

## Architecture

extending from the bombetta specification we have the following additional implementation details...

```solidity
/////////////////////////////// TYPES ////////////////////////////////////

struct ActiveJob {
    address requester; // address of the requester requesting the proof
    address prover; // address of the prover obligated to fufill the request
    uint256 deadline; // deadline of the request
    address token; // reward token
    uint256 requestReward; // request reward token amount
    uint256 proverStake; // eth amount the prover staked
    bytes32 publicInputsDigest; // hash of public input(s) for proof
    bytes32 provingSystemAuxDataCommitment; // aligned layer proving system commitment
}

struct Submission {
    bytes32 proofCommitment; // hash of proof that the proof provider submitted when resolving
    bytes32 batchMerkleRoot; // merkle root that the proof provider's submitted proof is included within
    uint256 verificationDataBatchIndex; // hash of the leaf node within the aligned layer merkle tree defined by the batchMerkleRoot
    bytes merkleProof; // merkle proof to prove inclusion of the leaf within the submitted root
}

////////////////////////////// CONSTANTS /////////////////////////////////

// Canonical Permit2 contract
ISignatureTransfer public immutable PERMIT2;
// Aligned Layer Service Manager contract
IAlignedLayerServiceManager public immutable ALIGNED_LAYER_SERVICE_MANAGER;

/////////////////////////////// STORAGE //////////////////////////////////

// mapping to active job data
// requestId -> ActiveJob
mapping(bytes32 => ActiveJob) public activeJobData;
```

As is common with other Bombetta market contract designs the `AlignedLayerBombetta` contract uses permit2 signatures to 
handle transferring of the proof request reward tokens into the bombetta market along with the eth stake from the proof 
provider who submitted a bid to a given proof request. The permit2 signature signed by the proof requester includes the 
request witness hash of data committed to alongside the permit transfer which pertains to the specifics of the proof 
request. refer to the below.

```solidity
struct RequestWitness {
    uint256 provingTime;
    uint256 nonce;
    address token;
    uint256 amount;
    address market;
    uint64 startTimestamp;
    uint128 minimumStake;
    BombettaMetadata meta;
    uint256 deadline;
}
```

The proof requester in the case of the `AlignedLayerBombetta` contract is using the BombettaMetadata.extraData field for
including the hash `provingSystemAuxDataCommitment`. This piece of data is included along with the `publicInputsDigest`
in order to correctly assert the inclusion within aligned layer's merkle root batches.

### Aligned Layer Merkle Proofs

Below are some Aligned Layer specific resources related to performing inclusion proofs...
- [Aligned Layer batcher merkle tree/leaf construction code](https://github.com/yetanotherco/aligned_layer/blob/main/batcher/aligned-batcher-lib/src/types/mod.rs)
- [Aligned Layer service manager on-chain inclusion proofs code](https://github.com/yetanotherco/aligned_layer/blob/main/contracts/src/core/AlignedLayerServiceManager.sol#L118)

Notice that the aligned layer protocol constructs `indexed merkle trees` periodically as proof submissions are
submitted and the associated batch merkle roots are posted onto ethereum available to check inclusion with.

Resolution of a Proof Request in the `AlignedLayerBombetta` adheres to the same high level flow of the Bombetta 
specification in that a proof request is only valid if the submitted proof (merkle proof in this specific case) is valid
and resolved before the deadline defined by ProofRequest.provingTime.

The way in which the proof submission is checked during resolution in the case of the `AlignedLayerBombetta` contract is
as follows...

```solidity
/// @dev check the correctness of the submitted proof during execution of resolve() by asserting inclusion
///      within a valid aligned layer proof batch
function _checkProofSubmission(
    bytes32 publicInputsDigest,
    bytes32 provingSystemAuxDataCommitment,
    address prover,
    bytes calldata opaqueSubmission // proofCommitment, batchMerkleRoot, verificiationDataBatchIndex, merkleProof
) internal returns (bool) {
    // decode submission data
    Submission memory submission = abi.decode(opaqueSubmission, (Submission));

    // compute leafHash for the given proof request based on proof requester's commitments and prover's `proofCommitment`
    bytes32 correctLeafHash = keccak256(
        abi.encodePacked(submission.proofCommitment, publicInputsDigest, provingSystemAuxDataCommitment, prover)
    );

    // check batch merkle root has an associated aligned layer service manager task that was responded to
    IAlignedLayerServiceManager.BatchState memory alignedLayerBatch =
        IAlignedLayerServiceManager(ALIGNED_LAYER_SERVICE_MANAGER).batchesState(submission.batchMerkleRoot);
    // the batch submitted maps to a task that has not been responded to, submission is invalid
    if (alignedLayerBatch.responded != true) return false;

    uint256 batchIndex = submission.verificationDataBatchIndex;
    bytes memory proof = submission.merkleProof;
    bytes32 computedHash = correctLeafHash;
    // this is an inclusion proof for an indexed merkle tree, where the leaf nodes are constructed like `correctLeafHash` above
    for (uint256 i = 32; i <= submission.merkleProof.length; i += 32) {
        if (batchIndex % 2 == 0) {
            // if ith bit of index is 0, then computedHash is a left sibling
            assembly {
                mstore(0x00, computedHash)
                mstore(0x20, mload(add(proof, i)))
                computedHash := keccak256(0x00, 0x40)
                batchIndex := div(batchIndex, 2)
            }
        } else {
            // if ith bit of index is 1, then computedHash is a right sibling
            assembly {
                mstore(0x00, mload(add(proof, i)))
                mstore(0x20, computedHash)
                computedHash := keccak256(0x00, 0x40)
                batchIndex := div(batchIndex, 2)
            }
        }
    }

    // Check if the computed hash (root) is equal to the provided root
    return computedHash == submission.batchMerkleRoot;
}
```
Following the _checkProofSubmission() function step by step we take in the hash of the public inputs 
`publicInputsDigest`, the hash/commitment to the proving system `provingSystemAuxDataCommitment`, and the address of the
prover `prover`. All the aforementioned data is determined at bid() time for a given proof request because both the
`publicInputsDigest` and the `provingSystemAuxDataCommitment` are commited to by the proof requester within the request
witness data of their proof request signature. And the prover address is simply the msg.sender that bid() on the proof 
requester's proof request. This data is then stored in the contract after the bid has been placed and the auction is
finished so it can be used at resolution time.

The prover must submit the encoded `Submission` struct mentioned above at the top of the architecture section within
the opaqueSubmission bytes calldata array. This data is required to confirm the submission of the correct proof to
aligned layer's off-chain verification service.

### Supported Interfaces

- Bombetta interface still holds here.

## Workflow

In order to move proof verification off-chain with the concrete use case of using aligned layer, the proof 
provider must bid on the proof request as is standard for all bombetta markets. Then they must follow these steps...

1. They compute the proof that the requester requires and submit it to aligned layer's API for off-chain verification.
2. Once the proof they are attempting to provide for the proof requester has been submitted to aligned layer they wait until the next batch root is submitted on-chain within the `AlignedLayerServiceManager.createNewTask()` function's execution.
3. Once the aligned layer service manager contract has the new task with the batch merkle root which includes the proof provider's submitted proof (assuming its a valid proof), the proof provider must wait for aligned layer to execute `AlignedLayerServiceManager.respondToTask()` which finalizes the batch's validity on-chain by trusting aligned layer's off-chain consensus around proof validity for that given batch of proofs (which includes the proof that the proof provider submitted before hand)
4. After the aligned layer service manager contract executes a response to the task which includes the specific batch merkle root the proof provider's individual proof is included within, the proof provider must call the `resolve()` function with the data the bombetta contract needs to assert it was included within one of aligned layer's proof batches to resolve the intent they bid upon and collect the reward to avoid being slashed.

# Status: functional with minimal testing
