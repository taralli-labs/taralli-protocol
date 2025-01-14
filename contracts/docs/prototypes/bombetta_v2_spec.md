# Bombetta Specification V2

## Summary

Bombetta V2 is a modification to the original bombetta spec which functions differently although similar in that its a
functional proving marketplace.

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
3. A proof provider sees the proof requests and chooses to bid on proof requests they deem viable and then submit a bid() call to the associated bombetta market that was commited to in the proof request intent. A `ProofRequestBidSubmitted` event is emitted and indexed/stored off-chain for resolution of the proof request later.
4. The proof provider who bid on the request now has an "active job" in that they are obligated to submit a valid proof by the deadline detailed by `ProofRequest.provingTime` to avoid being slashed.
5. The proof provider once they have computed the correct proof, submits the proof and the active job data they bid upon to the resolve() function. Assuming a valid proof the request is resolved with the proof provider being rewarded for submitting a correct proof. A `ProofRequestResolved` event is emitted.

### Visualization

![image](./images/bombetta.png)

## Events

```solidity
//////////////////////////// EVENTS //////////////////////////////////////

event ProofRequestBidSubmitted(bytes32 indexed requestId, bytes activeJobData);
event ProofRequestResolved(bytes32 indexed requestId, bytes32 jobHash);
```

## Types

For a given Proof Request the signed intent structure is the following...

```solidity
pragma solidity ^0.8.23;

/// @notice Represents the parameters of a `ProofRequest`'s verification logic.
/// @custom:field publicInputsDigest hash of public inputs for the proof verifier. The signed
///               intent commits to these public inputs in order to prevent the proof provider
///               being able to send different public inputs without getting slashed
/// @custom:field extraData Opaque data relating to market specific logic 
///               (e.g. necessary verification data)
struct BombettaMetadata {
    bytes32 publicInputsDigest;
    bytes extraData;
}

/// @notice Represents an opaque request for a proof to be generated.
/// @custom:field provingTime Amount of time to generate the proof. Period starts
///               when proof request bid is submitted.
/// @custom:field nonce The permit signature nonce.
/// @custom:field token The address of the ERC20 token that the request
///                     creator will reward the proof provider with.
/// @custom:field amount The amount of `token` that the request creator
///                      will pay the proof provider for a successful bid.
/// @custom:field market The address of the bombetta market that the
///                      proof request is being submitted to.
/// @custom:field startTimestamp The starting timestamp of the request auction.
/// @custom:field minimumStake The minimum amount of stake that the proof provider
///                            is required to provide when bidding on a request
///                            denominated in wei. This value is slashed if the
///                            proof request is not resolved before the deadline.
/// @custom:field meta The metadata of the request.
/// @custom:field deadline The deadline of the proof request auction & signature.
///               If no bid is submitted before this timestamp, the request is
///               invalid.
/// @custom:field signature The signature of the permit message with
///                         the witness data. Should authorize the
///                         transfer of `amount` `token` to this
///                         contract, up to and including `deadline`.
struct ProofRequest {
    // witness data
    uint256 provingTime;
    uint256 nonce;
    address token;
    uint256 amount;
    address market;
    uint64 startTimestamp;
    uint128 minimumStake;
    BombettaMetadata meta;
    uint256 deadline;
    // signature
    bytes signature;
}
```

This data structure is meant to handle all proof requests across all proving systems and all circuits that can/would
be verified on-chain (e.g. ethereum/evm).

The signature field can be used to detail any arbitrary signed intent logic but is currently concretely implemented for
the first few bombetta market contracts I have created as a permit2 signature including a commitment to a witness. The
witness data is the data contained in the proof request made/signed by the proof requester.

### Generic Interface

```solidity
/// @title  Bombetta, a permission-less proving marketplace
/// @author Taralli Labs
abstract contract Bombetta is IBombetta {
    /// @notice Place a bid for a signed proof request.
    /// @param request The request that is being bid upon.
    function bid(ProofRequest calldata request) external payable virtual returns (uint256) {}

    /// @notice Resolve a bid for a signed proof request.
    /// @param request The request that is being bid upon.
    /// @param activeJob The encoded active job data associated to the proof 
    ///                  request that was bid upon.
    /// @param opaqueSubmission The opaque data that will be decoded by the
    ///                         market contract and passed to the verifier.
    ///                         Empty if the deadline has been reached and
    ///                         the proof provider is being slashed.
    function resolve(ProofRequest calldata request, bytes calldata activeJob bytes calldata opaqueSubmission) external virtual {}
}
```

### function bid()

In the bid() fn the market checks the validity of the signature made by a given proof requester who signed the intent
specifically the field ProofRequest.signature. If the signature is valid the function proceeds with transferring any
value or stake necessary for the parties involved to accept the intent as successfully bid upon (e.g. token reward
and/or eth stake). After the execution of asset swaps the bid function then emits in the `ProofRequestBidSubmitted` 
event, any data specific to the request for it to be indexed off-chain and used at some later time by calling resolve().

a high level example of data that could be stored in a bombetta contract after a bid has been placed on a proof request
is below.

```solidity
struct ActiveJob {
    address requester;          // address of the requester that signed the ProofRequest that has now been bid upon
    address prover;             // address of the proof provider who bid on the ProofRequest successfully
    address token;              // address of the reward token the requester has transferred to this bombetta contract
    uint256 proverStake;        // eth amount the proof provider staked in this bombetta contract
    uint256 requestReward;      // request reward token amount
    bytes32 publicInputsDigest; // hash of public input(s) the requester has signed/committed to in their signature.
}

// requestId -> ActiveJob hashes
mapping(bytes32 => bytes32) public activeJobHashes;
```

In this aspect, a signed ProofRequest, once it has been bid() on is now an "active job", in that it has actual tokens 
from the requester and eth from the proof provider at risk within the bombetta market contract. There is now a time 
sensitive obligation for the proof provider to resolve the request by submitting a valid proof submission for the 
associated proof request or be slashed their eth stake for failing to do so by the proof request deadline.

### function resolve()

In the resolve fn, we check the validity of the proof submission for the given proof request which contains 4 possible
cases.

1. The proof provider submits a valid proof before the deadline of the request and receives the token reward + their eth stake back.
2. The proof provider submits an invalid proof before the deadline of the request and gets slashed their eth, sending both the token reward and slashed eth back to the requester.
3. The proof provider doesnt call the resolve() fn by the deadline of the proof request and can now be slashed if any address (external resolver) calls resolve() for this request returning the tokens back to the requester along with the slashed eth.
4. The caller of the resolve() fn submits an empty and/or non-sensical proof request which reverts.

Zooming in on the resolve() logic specifically in case #1, the proof submission inputs made by the proof provider to 
resolve() before the deadline are valid because the market asserts the specifics of the proving system and inputs used. 
This is the specificity needed to assert in the protocol that the exact proof request the proof requester wants based on
their intent/signature is fulfilled, where every other proof submission besides the one that matches, are slashed.