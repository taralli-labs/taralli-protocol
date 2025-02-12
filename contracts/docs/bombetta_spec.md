# Bombetta Specification

## Summary

The Bombetta specification describes a smart contract standard for verifiable compute marketplaces to implement that functions on the concept of
"proof requests" communicated by a requesting party to a set of available proof providers listening for these requests to see wether fulfilling
them is profitable or not. The standard leaves design details open such that fully decentralized and permission-less compute marketplaces are made 
possible, using custom pricing functions, immutable custody of assets, and resolution of requests all executed fully on-chain (or in some other fault-tolerant/trusted way).
Note that at any point in the implementation detail of the bombetta contract standard trust assumptions can be added as desired while still adhereing to the standard laid out below
assuming the interface is maintained.

## Agents

The Agents that involve themselves and take on various roles in the bombetta protocol are the following...

1. requester: An Ethereum account that signs a signature with some commitments to communicate an intent that they want some proof computed at a given price on their behalf by some deadline and submits it to an offchain pool where proof providers can see it.
2. provider: An Ethereum account that sees the request off-chain and sends in a bid to a bombetta contract's bid() function to secure the rights to resolve the requester's request. Further, provider's after bidding on a given proof request are obligated to resolve() it to avoid being slashed for failing to fulfill the request determined by the data describing the request (BombettaTypes.ProofRequest).
3. external resolver: An Ethereum account that calls resolve() in order to resolve a stale/unfufilled request that has not reached resolution but has passed its predefined deadline in order to slash the provider who originally committed to fulfilling the request.

## Example High Level User Flow

1. Requester signs an ECDSA signature (most commonly a permit2 signature) detailing a certain reward that can be swapped into the contract along with data about what proof they want computed by what time.
2. Requester sends the signature and data about the request's requirements to some network/group of providers so they can view the intent and determine its value.
3. A provider sees the requests and chooses to bid on the ones they deem viable and then submit a bid() transaction to the associated bombetta market that was commited to in the proof requests data.
4. The provider who bid on the request now has an "active request" in that they are obligated to compute and submit a valid proof by the deadline detailed by `provingTime` to avoid being slashed.
5. The provider once they have computed the correct proof submits the proof and the request is resolved with the provider being rewarded for submitting a correct proof or slashed if the proof is invalid.

## Types

For a given ProofRequest the signed intent structure is the following...

```solidity
/// @notice Represents an opaque request for a proof to be generated.
/// @custom:field signer The signer of the signature.
/// @custom:field market The address of the market that the proof request is being submitted to.
/// @custom:field nonce The permit signature nonce.
/// @custom:field rewardToken The address of the ERC20 token that the request creator/signer will reward the
///               proof provider with for resolving the request.
/// @custom:field maxRewardAmount The maximum amount of `token` that the request creator will pay the prover
///                               for a successful bid.
/// @custom:field minRewardAmount The minimum token amount available as a reward for bidding on the proof
///                               request. The reward will typically increase overtime until the full amount
///                               at the deadline of the signature is reached.
/// @custom:field minimumStake The minimum amount of stake that the prover is required to provide when bidding
///                            on a request denominated in wei. This value is slashed if the proof request is
///                            not resolved before the resolution deadline.
/// @custom:field startAuctionTimestamp The starting timestamp of the proof request auction.
/// @custom:field endAuctionTimestamp The deadline timestamp of the proof request auction & signature. If no
///                                   bid is submitted before this timestamp, the request is revoked.
/// @custom:field provingTime Amount of time (in seconds) to generate the proof. Time period starts when proof
///                           request auction is completed during bombetta.bid()'s execution.
/// @custom:field inputsCommitment Hash of all information the proof requester commits to in order to check
///                                that the opaque proof submission data at resolve() time is exactly what is
///                                requested. (for example the public inputs of the requested proof)
/// @custom:field extraData Opaque data relating to the bombetta market specific verification logic, if needed
///                         (verifier addr, fn selector, public inputs calldata location in opaqueSubmission, etc.)
struct ProofRequest {
    // general
    address signer;
    address market;
    uint256 nonce;
    // reward
    address rewardToken;
    uint256 maxRewardAmount;
    uint256 minRewardAmount;
    // stake requirements
    uint128 minimumStake;
    // time constraints
    uint64 startAuctionTimestamp;
    uint64 endAuctionTimestamp;
    uint32 provingTime;
    // verification commitments
    bytes32 inputsCommitment;
    bytes extraData;
}
```

This data structure is meant to handle all requests across all proving systems/proof types that can/would
be executed and verified on-chain (e.g. ethereum/evm).

The signature makes a commitment to all the fields within the proof request type alongside a permit2 signature but can in theory be used 
to detail any arbitrary signed intent logic for any ECDSA signature schema that uses eth accounts. Currently the main concrete implementation
showing this is the [universal bombetta](./universal_bombetta_spec.md) market contract. The witness data is simply the data contained in the proof 
request signed by the proof requester and the permit2 signature approach makes sense since the most readily available economic assets 
with the highest liquidity/value on ethereum/evm networks are typically ERC20 tokens (WETH, WBTC, USDC, USDT, etc.).

For more information on how permit2 signatures work check [here](https://github.com/Uniswap/permit2/).

### Generic Interface

```solidity
/// @title  Bombetta
/// @author Taralli Labs
/// @notice a permission-less verifiable compute marketplace
abstract contract Bombetta is IBombetta {
    /// @notice emitted when a bid is placed successfully on a signed proof request.
    event Bid(
        address indexed signer,
        bytes32 indexed requestId,
        address rewardToken,
        uint256 rewardAmount,
        uint256 ethStake,
        address provider
    );
    /// @notice emitted when a signed proof request is successfully resolved.
    event Resolve(address indexed signer, bytes32 indexed requestId, address resolver);

    /// @notice Place a bid for a signed proof request.
    /// @param request The proof request that is being bid upon.
    /// @param signature The signature of proof request.
    /// @return rewardToken The address of the reward token
    ///         rewardAmount The token reward amount available upon resolution.
    ///         provingDeadline The timestamp defining when the proof request must be resolved.
    function bid(ProofRequest calldata request, bytes calldata signature)
        external
        payable
        virtual
        returns (address, uint256, uint256)
    {}

    /// @notice Resolve a bid for a signed proof request.
    /// @param requestId The requestId associated to the ProofRequest being resolved.
    /// @param opaqueSubmission The opaque data that will be decoded by the market contract and passed to the verifier.
    ///                         Empty if the deadline has been reached and the prover is being slashed.
    /// @param partialCommitment The partial commitment to a field contained in opaqueSubmission needed to reconstruct
    ///                          that same final hash field within opaqueSubmission using another partial commitment
    ///                          value supplied by the proof requester before hand. (not always needed)
    /// @return providerResolved Value returning true if the original prover address that bid() to fulfill the request
    ///                        resolved the proof request. False if resolved by another address.
    function resolve(bytes32 requestId, bytes calldata opaqueSubmission, bytes32 partialCommitment)
        external
        virtual
        returns (bool providerResolved)
    {}
}
```

### function bid()

In the bid() function the market checks the validity of the signature made by a given requester who signed the 
intent. If the signature is valid the function proceeds with transferring any value or stake necessary for the parties
involved to accept, through the protocol's rules that the intent is successfully bid upon (e.g. token reward and/or eth stake). 
After the execution of asset transfers, the bid function then typically stores any data specific to the request for it to be 
resolved at some later time by calling resolve().

a high level example of data that could be stored in a bombetta contract after a bid has been placed on a proof request 
is below (universal bombetta)...

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

/// @notice mapping to active proof request data
// requestId -> ActiveProofRequest
mapping(bytes32 => ActiveProofRequest) public activeProofRequestData;
```

In this aspect, a signed ProofRequest, once it has been bid() on is now "active", in that it has actual tokens/economic value from the requester 
and eth from the provider at risk within the bombetta market contract. There is now a time sensitive obligation for the provider to resolve the 
request by submitting a valid proof for the associated request or be slashed their eth stake while receiving no reward for failing to do so by 
the proof request resolution deadline.

### function resolve()

In the resolve fn, we check the validity of the proof submission for the given request which contains 4 possible
cases.

1. The provider submits a valid proof before the deadline of the request and receives the token reward + their eth stake back.
2. The provider submits an invalid proof before the deadline of the request and gets slashed their eth, sending both the token reward and slashed eth back to the requester.
3. The provider doesnt call the resolve() fn by the deadline of the proof request and can now be slashed if any address (external resolver) calls resolve() for this request returning the tokens back to the requester along with the slashed eth.
4. The caller of the resolve() fn submits an empty and/or non-sensical proof request which reverts.

Zooming in on the resolve() logic specifically in case #1 (happy path), the proof submission made by the provider is a valid 
proof submitted before the deadline that also contains the same inputs/any other commitments that were stored 
from earlier within the bid() function call for this specific ProofRequest. So the protocol asserts the proving system, 
the circuit if required, the public inputs/other commitments that requester commits to. This is the specificity needed to assert 
the exact request the requester asked for based on their intent/signature is the only one that can be verified when the provider
calls resolve to try and get the reward.
