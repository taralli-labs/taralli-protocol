# Porchetta Specification

## Summary

The Porchetta specification describes a smart contract standard for verifiable compute marketplaces to implement that functions on the concept of
"proof offers" communicated by a proof providing party to a set of available proof requesters looking for proof offers to see wether buying them is 
profitable or not. The standard leaves design details open such that fully decentralized and permission-less compute marketplaces are made 
possible, using custom pricing functions, immutable custody of assets, and resolution of offers all executed fully on-chain (or in some other fault-tolerant/trusted way).
Note that at any point in the implementation detail of the porchetta contract standard trust assumptions can be added as desired while still adhereing to the standard laid out below
assuming the interface is maintained.

## Agents

The Agents that involve themselves and take on various roles in the bombetta protocol are the following...

1. provider: An Ethereum account that signs a signature with some commitments to communicate an intent that they will provide/compute a proof at a given price on the behalf of a requesting party that secures the rights to this offer.
2. requester: An Ethereum account that sees the offer off-chain and sends in a bid to a porchetta contract's bid() function to secure the rights to the provider's offer. Provider's, after a requester successfully bids on their proof offer, are obligated to resolve() it to avoid being slashed their stake for failing to fulfill the offer described by the data it is made up of (PorchettaTypes.ProofOffer).
3. external resolver: An Ethereum account that calls resolve() in order to resolve a stale/unfufilled offer that has not reached resolution but has passed its predefined deadline in order to slash the provider who originally committed to fulfilling the offer.

## Example High Level User Flow

1. Provider signs an ECDSA signature (most commonly a permit2 signature) detailing a certain proof workload they are offering at a given price in tokens which, during the bidding phase, can be swapped into the contract along with data about what proof needs to be computed by what time.
2. Provider sends the signature and data about the offer's requirements to some network/group of requesters so they can view the intent and determine its value.
3. A requester sees the offer of the provider and chooses to bid on the offers they deem viable and then submit a bid() transaction to the associated porchetta market that was commited to in the proof offers data.
4. The requester after bidding on the offer now creates an "active proof offer" for the provider in that the provider is now obligated to compute and submit the valid proof described in the offer by the deadline detailed by `provingTime` to avoid being slashed.
5. The provider once they have computed the correct proof submits the proof and the offer is resolved with the provider being rewarded for submitting a correct proof or slashed if the proof is invalid.

## Types

For a given ProofOffer the signed intent structure is the following...

```solidity
/// @notice Represents an opqaue offer for a proof to be generated
/// @custom:field signer The signer of the signature.
/// @custom:field market The address of the market that the proof offer is being submitted to.
/// @custom:field nonce The permit signature nonce.
/// @custom:field rewardToken The address of the ERC20 token that the offer creator/signer will be rewarded for
///                           resolving the offer.
/// @custom:field rewardAmount The price in tokens the proof provider will be paid
///               upon successfully resolving the proof offer.
/// @custom:field stakeToken The token that will be used as collateral for the proof offer.
/// @custom:field stakeAmount The amount of stake denominated in `stakeToken` that the proof provider is required
///                           to provide as collateral when a requester bids on their proof offer. This value is
///                           slashed if the proof offer is not resolved before the resolution deadline.
/// @custom:field startAuctionTimestamp The starting timestamp of the proof offer auction.
/// @custom:field endAuctionTimestamp The deadline timestamp of the proof offer auction & signature. If no bid is
///                                   submitted before this timestamp, the offer is revoked.
/// @custom:field provingTime Amount of time (in seconds) to generate the proof. Time period starts when proof
///                           offer auction is completed during bombetta.bid()'s execution.
/// @custom:field inputsCommitment Hash of all information the proof provider commits to in order to prove to the
///                                requesting party that they computed the proof that was promised in the offer.
///                                (for example the public inputs of the offered proof)
/// @custom:field extraData Opaque data relating to the Porchetta market specific verification logic, if needed
///                         (verifier addr, fn selector, public inputs calldata location in opaqueSubmission, etc.)
struct ProofOffer {
    // general
    address signer;
    address market;
    uint256 nonce;
    // price
    address rewardToken;
    uint256 rewardAmount;
    // stake requirements
    address stakeToken;
    uint256 stakeAmount;
    // time constraints
    uint64 startAuctionTimestamp;
    uint64 endAuctionTimestamp;
    uint32 provingTime;
    // verification commitments
    bytes32 inputsCommitment;
    bytes extraData;
}
```

This data structure is meant to handle all offers across all proving systems/proof types that can/would
be executed and verified on-chain (e.g. ethereum/evm).

The signature makes a commitment to all the fields within the proof offer type alongside a permit2 signature but can in theory be used 
to detail any arbitrary signed intent logic for any ECDSA signature schema that uses eth accounts. Currently the main concrete implementation
showing this is the [universal porchetta](./universal_porchetta_spec.md) market contract. The witness data is simply the data contained in the proof 
request signed by the proof provider and the permit2 signature approach makes sense since the most readily available economic assets 
with the highest liquidity/value on ethereum/evm networks are typically ERC20 tokens (WETH, WBTC, USDC, USDT, etc.).

For more information on how permit2 signatures work check [here](https://github.com/Uniswap/permit2/).

### Generic Interface

```solidity
/// @title  Porchetta
/// @author Taralli Labs
/// @notice a permission-less verifiable compute marketplace
abstract contract Porchetta is IPorchetta {
    /// @notice emitted when a bid is placed successfully on a signed proof offer.
    event Bid(
        address indexed signer,
        bytes32 indexed offerId,
        address rewardToken,
        uint256 rewardAmount,
        address stakeToken,
        uint256 stakeAmount,
        address requester
    );
    /// @notice emitted when a signed proof offer is successfully resolved.
    event Resolve(address indexed signer, bytes32 indexed offerId, address resolver);

    /// @notice Place a bid for a signed proof offer..
    /// @param offer The offer that is being bid upon.
    /// @param signature The signature of the proof offer.
    /// @return rewardToken the address of the reward token
    ///         rewardAmount The token reward amount available upon resolution.
    ///         provingDeadline The timestamp defining when the proof offer must be resolved.
    function bid(ProofOffer calldata offer, bytes calldata signature)
        external
        virtual
        returns (address, uint256, uint256)
    {}

    /// @notice Resolve a bid for a signed proof offer..
    /// @param offerId The offerId associated to the ProofOffer being resolved
    /// @param opaqueSubmission The opaque data that will be decoded by the
    ///                         market contract and passed to the verifier.
    ///                         Empty if the deadline has been reached and
    ///                         the proof provider is being slashed.
    /// @return proverResolved Value returning true if the original prover address that bid() to fulfill the request
    ///                        resolved the proof offer. Returning false if the provider failed to fulfill the request
    ///                        and the stale request is resolved by another address.
    function resolve(bytes32 offerId, bytes calldata opaqueSubmission) external virtual returns (bool proverResolved) {}
}
```

### function bid()

In the bid() function the market checks the validity of the signature made by a given provider who signed the 
intent. If the signature is valid the function proceeds with transferring any value or stake necessary for the parties
involved to accept through the protocol's rule, that the intent is successfully bid upon (e.g. token reward and/or token stake). 
After the execution of asset transfers, the bid function then typically stores any data specific to the offer for it to be 
resolved at some later time by calling resolve().

a high level example of data that could be stored in a bombetta contract after a bid has been placed on a proof request 
is below (universal bombetta)...

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

// mapping to active proof offer data
// offerId -> ActiveProofOffer
mapping(bytes32 => ActiveProofOffer) public activeProofOfferData;
```

In this aspect, a signed ProofOffer, once it has been bid() on by a requester it is now "active", in that it has actual tokens/economic value from the requester 
and provider at risk within the porchetta market contract. There is now a time sensitive obligation for the provider to resolve the offer a given requester accepted
by submitting a valid proof for the associated offer or be slashed their token stake while receiving no reward for failing to do so by the proof offer resolution deadline.

### function resolve()

In the resolve fn, we check the validity of the proof submission for the given offer which contains 4 possible
cases.

1. The provider submits a valid proof before the deadline of the offer and receives the token reward + their eth stake back.
2. The provider submits an invalid proof before the deadline of the offer and gets their stake tokens slashed, sending both the token reward and slashed tokens back to the requester.
3. The provider doesnt call the resolve() fn by the deadline of the proof offer and can now be slashed their tokens if any address (external resolver) calls resolve() for this offer returning the tokens back to the requester along with the slashed provider tokens.
4. The caller of the resolve() fn submits an empty and/or non-sensical proof request which reverts.

Zooming in on the resolve() logic specifically in case #1 (happy path), the proof submission made by the provider is a valid 
proof submitted before the deadline that also contains the same inputs/any other commitments that were stored from earlier within 
the bid() function call for this specific ProofOffer. So the protocol asserts the proving system, the circuit if required, 
the public inputs/other commitments that the provider commits to. This is the specificity needed to assert the exact offer the 
requester is buying from the provider based on the provider's intent/signature is the only scenario that can be verified to true 
when the provider calls resolve to try and get the requester's reward.   
