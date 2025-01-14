# Bombetta Specification

## Summary

The Bombetta specification describes a smart contract standard for proof marketplaces that can function on a spectrum 
where fully decentralized and permission-less proving marketplaces are made possible, in that the pricing of proof 
intents, custody of assets, and resolution of proof requests is executed on-chain (or in some other fault-tolerant way).
At any point in the implementation detail of the bombetta contract trust assumptions can be added as desired while still
adhereing to the standard laid out below.

## Agents

The Agents that involve themselves and take on various roles in the bombetta protocol are the following...

1. proof requester: An Ethereum account that signs a signature to communicate an intent that they want some proof computed at a given price on their behalf by some deadline and submits it to some proof request mempool where proof providers can see it.
2. proof provider: An Ethereum account that submits a valid signed proof request from a proof requester that they saw in a proof request mempool to a bombetta contract's bid() function. Further proof provider's once they bid on a given proof request are obligated to resolve() to avoid being slashed for failing to fulfill the request.
3. external resolver: An Ethereum account that calls resolve() in order to resolve a stale request that has not reached resolution but has passed its predefined deadline in order to slash the proof provider who originally committed to fulfilling the proof request.

## Example High Level User Flow

1. Proof requester signs a permit2 signature detailing a certain token reward that can be swapped into the bombetta contract along with data about what proof they want computed by what time.
2. Proof requester sends the signature and data about the proof request's requirements to some network/group of proof providers so they can view the intent.
3. A proof provider sees the proof requests and chooses to bid on proof requests they deem viable and then submit a bid() call to the associated bombetta market that was commited to in the proof request intent.
4. The proof provider who bidded on the request now has an "active request" in that they are obligated to submit a valid proof by the deadline detailed by `ProofRequest.provingTime` to avoid being slashed.
5. The proof provider once they have computed the correct proof submits the proof and the request data they bid upon to the resolve() function and the request is resolved with the proof provider being rewarded for submitting a correct proof.

### Visualization

![image](./images/bombetta.png)

## Types

For a given Proof Request the signed intent structure is the following...

```solidity
pragma solidity ^0.8.23;

/// @notice Represents an opaque request for a proof to be generated.
/// @custom:field market The address of the bombetta market that the
///                      proof request is being submitted to.
/// @custom:field nonce The permit signature nonce.
/// @custom:field token The address of the ERC20 token that the request
///                     creator will reward the prover with.
/// @custom:field maxRewardAmount The maximum amount of `token` that the request creator
///                               will pay the prover for a successful bid.
/// @custom:field minRewardAmount The minimum token amount available as a reward for
///                               bidding on the proof request. The reward will
///                               typically increase overtime until the full amount
///                               at the deadline of the signature is reached.
/// @custom:field minimumStake The minimum amount of stake that the prover
///                            is required to provide when bidding on a request
///                            denominated in wei. This value is slashed if the
///                            proof request is not resolved before the deadline.
/// @custom:field startTimestamp The starting timestamp of the request auction.
/// @custom:field deadline The deadline of the proof request auction & signature.
///               If no bid is submitted before this timestamp, the request is
///               revoked.
/// @custom:field provingTime Amount of time to generate the proof. Period starts
///               when proof request auction is complete.
/// @custom:field publicInputsDigest hash of public inputs for proof verification.
/// @custom:field extraData Opaque data relating to market specific verification logic
/// @custom:field meta The metadata of the request.
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

This data structure is meant to handle all proof requests across all proving systems and all circuits that can/would
be verified on-chain (e.g. ethereum/evm).

The signature field passed into the bid/resolve functions can be used to detail any arbitrary signed intent logic but 
is currently concretely implemented for the universal bombetta market contract I have created as a permit2 signature 
including a commitment to a witness. The witness data is the data contained in the proof request made/signed by the proof 
requester.

### Generic Interface

```solidity
/// @title  Bombetta
/// @author Taralli Labs
/// @notice a permission-less proving marketplace
abstract contract Bombetta is IBombetta {
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

```

### function bid()

In the bid() function the market checks the validity of the signature made by a given proof requester who signed the 
intent. If the signature is valid the function proceeds with transferring any value or stake necessary for the parties
involved to accept the intent as successfully bid upon (e.g. token reward and/or eth stake). After the execution of asset
transfers the bid function then typically stores any data specific to the request for it to be resolved at some later 
time by calling resolve().

a high level example of data that could be stored in a bombetta contract after a bid has been placed on a proof request 
is below (universal bombetta)...

```solidity
struct ActiveJob {
    address requester; // address of the requester requesting the proof
    address prover; // address of the prover obligated to fufill the request
    uint256 deadline; // deadline of the request
    address token; // reward token
    uint256 requestReward; // request reward token amount
    uint256 proverStake; // eth amount the prover staked
    bytes32 publicInputsDigest; // hash of public input(s) for proof
    bytes verifierDetails; // data specific to verification (address, function selector, etc.)
}

// requestId -> ActiveJob
mapping(bytes32 => ActiveJob) public activeJobData; 
```

In this aspect, a signed ProofRequest, once it has been bid() on is now "active", in that it has actual tokens from the
proof requester and eth from the proof provider at risk within the bombetta market contract. There is now a time sensitive 
obligation for the proof provider to resolve the request by submitting a valid proof submission for the associated proof 
request or be slashed their eth stake for failing to do so by the proof request deadline.

### function resolve()

In the resolve fn, we check the validity of the proof submission for the given proof request which contains 4 possible
cases.

1. The proof provider submits a valid proof before the deadline of the request and receives the token reward + their eth stake back.
2. The proof provider submits an invalid proof before the deadline of the request and gets slashed their eth, sending both the token reward and slashed eth back to the requester.
3. The proof provider doesnt call the resolve() fn by the deadline of the proof request and can now be slashed if any address (external resolver) calls resolve() for this request returning the tokens back to the requester along with the slashed eth.
4. The caller of the resolve() fn submits an empty and/or non-sensical proof request which reverts.

Zooming in on the resolve() logic specifically in case #1, the proof submission made by the proof provider is a valid 
proof submitted before the deadline that also contains the same public inputs/any other commitments that were stored 
from earlier within the bid() function call for this specific ProofRequest. So the protocol asserts the proving system, 
the circuit and the public inputs/other commitments. This is the specificity needed to assert the exact proof request 
the proof requester wants based on their intent/signature, where every other proof submission besides the one with a 
matching proving system, circuit and public input/other commitments is slashed.
