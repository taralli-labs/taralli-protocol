# Taralli Design Specification

## ---------- Overview ----------

The taralli protocol is made up of 6 components.

1. marketplace smart contract(s)
2. primitives
3. server
4. client
5. worker
6. binaries

Currently many parts of the protocol are complete but in pre-alpha/alpha stage with minimal testing as well as some parts being incomplete and still a work in progress. Keep that in mind, all this software is provided as is with no guarantees. For more information on future plans read the [roadmap doc](./roadmap.md).

### --- Marketplace smart contract(s) ---

- [Bombetta specification](../contracts/docs/bombetta_spec.md)
- [UniversalBombetta specification](../contracts/docs/universal_bombetta_spec.md)
- [Porchetta specification](../contracts/docs/porchetta_spec.md)
- [UniversalPorchetta specification](../contracts/docs/universal_porchetta_spec.md)

A brief summary for those who dont read through the smart contract specs. As of now the compute intent types are market specific, meaning the ComputeRequests and their signed ProofRequests are commited to the bombetta market. The ComputeOffers and their signed ProofOffers are commited to the Porchetta market. This is subject to change based on new smart contract changes/additions.

### --- Taralli Primitives ---

Taralli primitives contains traits/types describing each supported system as well as types/utilities that are shared between the server and the clients.

#### Systems

All systems known by the users of the taralli protocol are organized by their respective IDs within the primitive crate's systems module. Clients can use these types to verbosely describe the computational workload they are requesting/offering to willing buyers. Thus, allowing other parties looking at these data structures at runtime through the protocol's clients to assess their correctness/value. This is done in order to minimize any runtime analysis that might be needed by opposing parties. Although, system specifications through the primitives crate are not necessarily a requirement for using the protocol it is very practical and highly advised to allow the bidding party (requesters & providers) some way, before they take risk by engaging in an auction to secure the request/offer, to have as close to 100% confidence the intent represents what they are wlling to participate in. This goes for all clients looking to buy/sell specific compute workloads.

The process to add a new complete system into the taralli protocol varies based on the system, but the first step is always to include the addition of the new impl of the `System` trait across a given struct which includes the desired inputs for the computational workload that system describes.

here is the system trait

```rust
pub trait System: Send + Sync + 'static + Clone + Serialize + for<'de> Deserialize<'de> {
    type Config: SystemConfig;
    type Inputs: Debug + Clone;

    fn system_id(&self) -> SystemId;
    fn config(&self) -> &Self::Config;
    fn inputs(&self) -> SystemInputs;
    fn validate_inputs(&self) -> Result<()>;
    fn system_params(&self) -> Option<&SystemParams> {
        None
    }
}
```
If the system does not have a common verification pattern then the implementation is complete with just the System impl and can be placed within the system id/system params enums. Further, if the system has a common verification pattern then system specific verifier constraints can also be implemented as detailed below. Verifier constraints are a compile time element of how the compute intents can be verified upon receipt depending on what common understanding the system has defined (more info in the intents validation section). The `CommonVerifierConstraints` details common fields that will present themselves across multiple systems which are implemented across various intent types that are made to be generic over all systems. For example, many proving systems have production/testnet verification contracts/functions that are to be used by those generating proofs of computation with the system in question. Because of this, it makes sense to define system specific constraints on verification to save people time/effort in analyzing which intents they are searching for.

```rust
/// Common verifier constraints across all intent types
pub trait CommonVerifierConstraints: Default + Debug + Clone {
    fn verifier(&self) -> Option<Address>;
    fn selector(&self) -> Option<FixedBytes<4>>;
    fn inputs_offset(&self) -> Option<U256>;
    fn inputs_length(&self) -> Option<U256>;
}
```

All systems the protocol server uses are fed into the systems macro with there enum name, as_str() string value, concrete systems struct and bit position for use in representing the system id as a mask

```rust
systems! {
    (Arkworks, "arkworks", ArkworksProofParams, 0x01),
    (Risc0, "risc0", Risc0ProofParams, 0x02),
    (Sp1, "sp1", Sp1ProofParams, 0x04)
}
```

#### Compute Intents

Primitives defines the high level trait `ComputeIntent`, which describes the structure of the intent types that implement it (ComputeRequest & ComputeOffer currently). The `system_id` is the way in which markets for particular compute workloads are organized/identified. The `system` field is meant to correspond to the system ID and describes the specifics of the compute workload that must be performed according to the intent (binaries, inputs, etc.). The `proof_commitment` describes the data to be used as a witness to the signature (permit2 signature) which binds tokens/eth to the obligations laid out by the intent, such that if they are not fulfilled or if they are tried to be broken/tampered with then financial consequences are executed via the smart contract protocol. On the other hand, this same commitment provides the means for reward if the obligations of both parties described by the intent are fulfilled. And lastly, the signature which as detailed above is the signature signed by the intent signer's private key which which commits to the proof commitment data. Below is the compute intent trait.

```rust
// Common trait for shared fields across all intent type's proof commitment structures
pub trait CommonProofCommitment: Serialize + for<'de> Deserialize<'de> + Send + Sync {
    fn market(&self) -> &Address;
    fn nonce(&self) -> &U256;
    fn start_auction_timestamp(&self) -> u64;
    fn end_auction_timestamp(&self) -> u64;
    fn proving_time(&self) -> u32;
    fn inputs_commitment(&self) -> FixedBytes<32>;
}

/// Trait representing common behavior for compute intents
pub trait ComputeIntent: Serialize + for<'de> Deserialize<'de> + Send + Sync {
    type System: System;
    type ProofCommitment: CommonProofCommitment;

    /// Compute Intent data
    fn system_id(&self) -> SystemId;
    fn system(&self) -> &impl System;
    fn proof_commitment(&self) -> &Self::ProofCommitment;
    fn signature(&self) -> &PrimitiveSignature;

    /// utility methods
    // type string associated to this intent type
    fn type_string(&self) -> String;
    // compute intent id
    fn compute_id(&self) -> FixedBytes<32>;
    // compute permit2 digest for intent signing
    fn compute_permit2_digest(&self) -> FixedBytes<32>;
}
```

The process of implementing a new compute intent for the protocol is to make a new ComputeIntent impl that uses existing associated type impls for the System and ProofCommitment traits or leverages new ones. For now, the 2 main implementations in use are the compute request and the compute offer with plans to add in new types/compositions of intents later. Reqeusts and offers are the 2 fundamental aspects of a market's supply chain so we will build out the protocol with these in mind to start. Furthermore, you can imagine more complicated intent structures such as recurring requests/offers, which can be referred to as "Intent Chains" or something along those lines. Intents that lay out challenges/competition parameters or revolve around speed of completion, and other various ideas depending on the needs/nature of a given compute market and its participants.

##### ComputeRequest

The `ComputeRequest` type is the structure used when submitting requests for compute to the server using the submit api endpoint 
by requester clients and broadcasted through streams to corresponding subscribers (streaming clients) who are listening for incoming 
compute requests.

```rust
/// Compute request type generic over all Systems
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(bound = "S: System")]
pub struct ComputeRequest<S: System> {
    pub system_id: SystemId,
    pub system: S,
    pub proof_request: ProofRequest,
    pub signature: PrimitiveSignature,
}
```

`ProofRequest` is generated by alloy's sol macro and is the rust representation of the proof commitment structure used as witness within the
compute request signature schema to tie the compute request to the bombetta market contract.

```rust
/// sol! Macro expansion result UniversalBombetta::ProofRequest
/// This is the rust representation of the onchain solidity type that provides
/// the outline for what values are commited to by an eth account's ECDSA 
/// signature, reference the bombetta smart contract spec for more info
#[allow(non_camel_case_types, non_snake_case)]
pub struct ProofRequest {
    pub signer: ::alloy::sol_types::private::Address,
    pub market: ::alloy::sol_types::private::Address,
    pub nonce: ::alloy::sol_types::private::U256,
    pub token: ::alloy::sol_types::private::Address,
    pub maxRewardAmount: ::alloy::sol_types::private::U256,
    pub minRewardAmount: ::alloy::sol_types::private::U256,
    pub minimumStake: u128,
    pub startAuctionTimestamp: u64,
    pub endAuctionTimestamp: u64,
    pub provingTime: u32,
    pub publicInputsCommitment: ::alloy::sol_types::private::FixedBytes<32>,
    pub extraData: ::alloy::sol_types::private::Bytes,
}
```

##### ComputeOffer

The `ComputeOffer` type is the structure used when submitting an offer of computation to the server using the corresponding submit endpoint
by offering clients and then stored within the server's postgres db for searching clients querying the server for quotes on various compute 
offerings.

```rust
/// Compute offer type generic over all Systems
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(bound = "S: System")]
pub struct ComputeOffer<S: System> {
    pub system_id: SystemId,
    pub system: S,
    pub proof_offer: ProofOffer,
    pub signature: PrimitiveSignature,
}
```

`ProofOffer` is generated by alloy's sol macro and is the rust representation of the profo commitment structure used as witness within the
compute offer signature schema to tie the compute offer to the porchetta market contract.

```rust
/// sol! Macro expansion result UniversalPorchetta::ProofOffer
/// This is the rust representation of the onchain solidity type that provides
/// the outline for what values are commited to by an eth account's ECDSA 
/// signature, reference the bombetta smart contract spec for more info
#[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
pub struct ProofOffer {
    pub signer: ::alloy::sol_types::private::Address,
    pub market: ::alloy::sol_types::private::Address,
    pub nonce: ::alloy::sol_types::private::primitives::aliases::U256,
    pub rewardToken: ::alloy::sol_types::private::Address,
    pub rewardAmount: ::alloy::sol_types::private::primitives::aliases::U256,
    pub stakeToken: ::alloy::sol_types::private::Address,
    pub stakeAmount: ::alloy::sol_types::private::primitives::aliases::U256,
    pub startAuctionTimestamp: u64,
    pub endAuctionTimestamp: u64,
    pub provingTime: u32,
    pub inputsCommitment: ::alloy::sol_types::private::FixedBytes<32>,
    pub extraData: ::alloy::sol_types::private::Bytes,
}
```

#### Intent Validators

Primitives crate also provides intent validators which serve to validate an intent upon first processing it as a client or as the protocol server. This is necessary due to the nature of how the compute market works. The intent signer and the intent bidder need to ensure that the intent they planned to provide/bid on in abstract makes sense as well as concretely maps to the specific constraints the intent outlines. At a high level, this can be seen as compute requesters being required to make requests that are possible to fulfill/correct in structure relative to what they identify them with (System ID, System, market data, etc) as well as providers for compute requests being required to fulfill the compute request without fail within its commited parameters to receive a reward or face economic consequences. Here is the trait...

```rust 
/// Trait for validating compute intents
pub trait IntentValidator<I: ComputeIntent>: Send + Sync {
    type ValidationConfig: CommonValidationConfig;
    type VerifierConstraints: CommonVerifierConstraints;

    /// Get the validation configuration
    fn validation_config(&self) -> &Self::ValidationConfig;
    /// Get the verifier constraints
    fn verifier_constraints(&self) -> &Self::VerifierConstraints;

    /// Validate the intent with the given parameters
    fn validate(&self, intent: &I, latest_timestamp: u64, market_address: &Address) -> Result<()> {
        // Full validation logic
        validate_system(intent, &self.validation_config().supported_systems())?;
        validate_market_address(intent.proof_commitment().market(), market_address)?;
        validate_time_constraints(
            intent.proof_commitment().start_auction_timestamp(),
            intent.proof_commitment().end_auction_timestamp(),
            intent.proof_commitment().proving_time(),
            latest_timestamp,
            self.validation_config().minimum_proving_time(),
            self.validation_config().maximum_start_delay(),
        )?;
        validate_nonce()?;
        self.validate_specific(intent)
    }

    /// Validate intent-specific constraints
    fn validate_specific(&self, intent: &I) -> Result<()>;
}
```

Each intent validator implementation is used by the clients to validate incoming intents that are streamed/queried to them or as a pre-submission check to confirm correctness before submission. When adding a new intent to the protocol a corresponding validator is needed.

### --- Taralli Server ---

The Taralli server hosts an api server that exposes endpoints to facilitate the communication of compute intents between market participants

#### Submit:

The submit endpoints allow compute intent signers to submit compute intents, have them partially validated by the server, and then
gossiped from the server over websocket streams or by postgres query to the clients listening/searching for compute intents to bid upon/fulfill.

##### Validation Overview:

The server's validation criteria is currently to do a partial validation across every aspect of the compute intents except the system data which remains compressed due to its larger size. The server validation
is open to interpretation and could be changed to validate less or more of the intents that are submitted based on the performance/load requirements caused from validating intents. Right now, it makes sense to cover
most of the validation for users of the protocol as a convenience if they want to rely on it then later decide wether the protocol server will provide validation as a service or not for various compute markets.

A viable intent has the following properties:
- the `system_id` exists
- all `proof_commitment` fields make sense given the accompanying intent data
    - `market` is the real deployed market contract the server is aware of.
    - `nonce` has not already been used.
    - `startAuctionTimestamp` is already reached or going to be reached in less than a time delay set by the server
    - `endAuctionTimestamp` has not passed the current timestamp and there is still time to submit a bid for the request.
    - `proving time` is a long enough duration given computational complexity of the given compute workload & proof.
    - `extra data` needs to make sense given the nature of the intent.
        - the extra_data field can be decoded as the VerifierDetails structs associated to the market contract.
        - the decoded verifier details can be checked against the system's corresponding verifier constraints type, if the submitted details adheres to the constraints then it is valid
- the provided `signature` is valid based on the provided `signer` address & proof commitment data


if one of these properties inside the validation layer of the server is checked and found not to hold then the server omits the intent
and does not add it to the validated pool that will be sent out to those listening for new submissions.

Overall the conditions above attempt to comprehensively layout the possible common needs/checks to perform, to be sure the server has already validated most if not all cases to filter out intents that do not make sense to look at bidding/taking economic risk on excluding the system data which will be done by the clients due to it being compressed and possibly very large.

#### Subscribe:

The Subscribe endpoint allows clients to subscribe to specific systems based on system IDs that they are willing to receive incoming intent submission notifications for over websocket streams. 
This is so the clients can bid on incoming intents seen from the protocol server there by using the market contract's bid() function to secure the exclusive right to perform the compute and 
generate the proof for that request within the agreed upon deadline. Then later resolve the intent using the market's resolve() function to get rewarded for correctly fulfilling it.

#### Query

The Query endpoint allows clients to search through the server's compute intent database by system ID. A method by which clients look for specific compute services they can bid on/buy through the market contracts.

#### Server Technical Overview

The Taralli server is a rust axum api server that uses the taralli primitives crate along with various internal modules
to execute all the functionality encapsulated by the 2 api routes (submit and subscribe).

```rust
/// lib.rs view of the taralli server crate
pub mod config; // configuration for server startup
pub mod error; // server specific errors
pub mod extracted_intents; // extraction logic for intents to be handled in their compressed/partial form
pub mod postgres; // postgres database logic for intent storage
pub mod routes; // handler fn modules for the submit, subscribe, and query endpoints
pub mod state; // central state types of the server
pub mod subscription_manager; // logic around maintaining websocket streams to subscribers of various system IDs
pub mod validation; // partial validation logic performed by the server upon any intent being submitted to it
```

An example axum server rust binary is [here](../bin/server/src/bin/server.rs)

### --- Taralli Client ---

The consumers (clients) of the taralli protocol require various tools/actions to facilitate the full end to end lifecycle of the compute intent process with ease of integration/interactivity. 
The client crate provides the tool chain that clients need in order to compose client binaries to participate effectively in the taralli marketplace in any given role. The abstract goal of 
the protocol is to coordinate those who need compute with those who have compute together such that the price of the desired compute workloads is discovered as fast and as accurately as possible.

#### Client Roles

Current client roles:
    - requester requesting client: submits requests for compute to the server and tracks the state of the request
    - requester searching client: submits queries to the server's intent database for compute offers, searches through them and bids on one, thereafter tracking the state of the offer
    - provider streaming client: subscribes to the server across a set of system ID(s), then awaits incoming compute requests to process and potentially fulfill.
    - provider offering client: submits an offer for compute to the server and tracks the state of the offer until its bid upon, thereafter fulfilling the offer.

#### Submit (client logic)

##### High level requirements needed to submit a compute intent

- use the intent builder for the intent type in question, then sign it with an eth account that has the asscoiated funds needed based on the intent's proof commitment.
- call the submit endpoint corresponding to the intent type in the taralli server and receive a response back denoting acceptance/rejection of the submission.
- upon confirmation of a submitted intent being validated by the server, the client opens up an event filter to track the status of that intent's auction status in the market contract (Bid() event).
  - This stage could result in either a successful bid event to move on to the resolution phase or no successful bid after the end time of the auction timestamp has passed and then a timeout/exit state occurs.
- Upon confirmation of a successful bid on the submitted intent, the client either opens up an event filter to track the resolve() event in the market contract or the client is providing compute and starts the worker execution and resolver themselves.
  - This stage in the client's execution can result in an intent that is resolved with a valid proof of the computation and the accompanying reward is given to the compute provider's eth account (no slashing penalty) or the case that provider did not provide a valid proof of the requested compute and must be slashed their eth stake plus no token reward (slashing can happen from an invalid proof or deadline timestamp being reached).

Once the intent has been resolved and the client either sees this confirmed through the market contract event filter (Resolve() event) if buying compute or they themselves are resolving the intent and get back a successful call to the resolve method of the market contract after finishing the work involved in the compute intent.

##### Requirements needed for proof_commitment (includes signature and signer addr)

use the `intent builder` for the given intent type (ComputeRequest or ComputeOffer)

```rust
/// core builder trait
pub trait IntentBuilder {
    type Intent;
    fn build(&self) -> Result<Self::Intent>;
}
```

intent builder requirements:
- ethereum rpc provider, to fetch state about the network (`current timestamp` of latest block and `account nonce`, perhaps other stuff?)
    - used to compute a reasonable, `startAuctionTimestamp` for the request
    - same for the `endAuctionTimestamp` when the auction is over and no more bids are allowed
- ethereum account to make signatures with, `signer & signature`
- deployed/active market contract address, `market`
- reward token contract address, `rewardToken`
- proving time allowed to the auction winner, `provingTime`
- the inputs commitment to the proof they want computed `inputsCommitment`
- extra data needed for committing to the verification data of the request in the case of the market contract, `extraData`
- and other fields that are intent type specific such as reward/stake and other proof/verification related parameters

extraData in the context of the 2 current market contracts (UniversalBombetta & UniversalPorchetta) is an encoded representation of each market's VerifierDetails solidity structs which have rust representations
that the client's intent builders will use to help in building the intent's proof_commitment verification data. These structs adhere to the below trait and goes hand in hand with the verifier constraints as the verifier details
are validated by their constraints.

```rust
/// Common verifier constraints across all intent types
pub trait CommonVerifierConstraints: Default + Debug + Clone {
    fn verifier(&self) -> Option<Address>;
    fn selector(&self) -> Option<FixedBytes<4>>;
    fn inputs_offset(&self) -> Option<U256>;
    fn inputs_length(&self) -> Option<U256>;
}
```

here is an example of the ComputeRequest verifier constraints and verifier details structs below.

```rust
/// Verifier constraints specific to ProofRequest proof commitments withing ComputeRequest intents
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RequestVerifierConstraints {
    pub verifier: Option<Address>,
    pub selector: Option<FixedBytes<4>>,
    pub is_sha_commitment: Option<bool>,
    pub inputs_offset: Option<U256>,
    pub inputs_length: Option<U256>,
    pub has_partial_commitment_result_check: Option<bool>,
    pub submitted_partial_commitment_result_offset: Option<U256>,
    pub submitted_partial_commitment_result_length: Option<U256>,
    pub predetermined_partial_commitment: Option<B256>,
}
```
```solidity
// solidity struct that gets encoded into the extraData field of the ComputeRequest market (Bombetta)
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

In order for the client to know what they need to submit in the extraData field they need access to...
1. the market's verification parameters type describing the decoded bytes of extraData (UniversalBombetta's VerifierDetails struct for example, same name, different fields in teh porchetta market as well)
2. all the individual pieces of data inside this encoded type (in the case of UniversalBombetta the verifier address, function selector of verifying function, etc. listed above)
3. most commonly the commitments here will be made up of the common verifier constraints hence why the trait is defined in this way. The verifier address, function selector and inputs field used by the market are the main fields to commit to as an intent signer wether making a compute request or compute offer.

High level examples... 

A user wants to request a groth16bn128 proof based on circuit A, with private and/or public inputs B, so they need some verifier
contract deployed on ethereum deployed/setup for circuit A, this verifer contract will have a certain contract address,
function signature, inputs commitment, submitted inputs offset/length and potentially other commitments that
need to be known by the requester in order for them to make the request that commits to those specific verifier 
details in the extraData field of the OnChainProofRequest to make the request to the exact proof they need.

A user wants to request a zkVM proof (e.g. risc0 zkVM) based on zkVM program A and zkVM inputs B, so they need a zkVM verifier
contract deployed on Ethereum at a given address that can be committed to by the requester along with the commitments 
to the function selector, inputs commitment (if needed), submitted inputs offset/length, potentially other
commitments that need to be known by the requester in order for them to make the request that commits to those specific verifier 
details in the extraData field of the OnChainProofRequest to make the request to the exact proof they need.

##### Requirements needed for the System field of the intent

use the `intent builder`

- `system_id` obtained by taralli primitives crate system definitions
- `system` also obtained by taralli primitives crate system definitions

Once the system information type has been laid out, the client fills in the necessary data for this struct using whatever tools necessary given their
understanding of the compute intent they are making to give the other clients the necessary information they need to understand and/or perform the compute task and 
prove it in the process.

##### Requirements for signature fields

signing is handled withing the client modules of each client that builds/signs compuet intents. Once an intent is fully built it can then be signed shortly after being submitted to the server.

##### tracking intent submissions

Once an intent is submitted the process to track it starts first with the intent auction phase which is essentially waiting for a successful bid to be placed on the intent wether it is a request or offer for compute. This is done within the client's tracker logic (trait below).

```rust
#[async_trait]
pub trait IntentAuctionTracker {
    type Intent;
    type BidEvent;
    async fn track_auction(
        &self,
        intent_id: FixedBytes<32>,
        timeout: Duration,
    ) -> Result<Option<Self::BidEvent>>;
}

#[async_trait]
pub trait IntentResolveTracker {
    type Intent;
    type ResolveEvent;
    async fn track_resolve(
        &self,
        intent_id: FixedBytes<32>,
        timeout: Duration,
    ) -> Result<Option<Self::ResolveEvent>>;
}
```

The auction tracker opens an event filter for the `Bid()` event at the associated intent id to see if a successul bid transaction was submitted within the market contract the intent commits to within its signature. When the bid transaction is successfully included in a valid block the event notifies the signer and the intent moves from the auction phase to the resolution phase. If the intent is from a requesting party (such is the case with compute requests) then another event filter is open tracking the `Resolve` event to track what happens during the resolution phase and if the request for compute ends up resolving correctly. On the other hand, if the intent comes from a providing party (such is the case with compute offers) then the tracking finishes and intiates the compute worker within the client so their intent can be resolved with a reward and no penalty.

#### Subscribe (client logic)

##### High level requirements needed to subscribe to a given market or set of markets

using system id(s) for each compute market from taralli primitives as input, call the subscribe endpoint of the server and receive a websocket stream back.

##### High level requirements needed to check intent viability

use the `intent analyzer` in order to use/implement various checks for certain properties on incoming intents 
coming from an existing market.

```rust
/// core analyzer trait
#[async_trait]
pub trait IntentAnalyzer {
    type Intent;
    async fn analyze(&self, latest_ts: u64, intent: &Self::Intent) -> Result<()>;
}
```

NOTE: the taralli server's validation layer will handle some basic checks besides more abstract qualities that should be decided by the clients,
such as what reward tokens and minimum prices are acceptable and potentially other logic as well that relates to the economic properties of the intent.

##### High level requirements needed to send bids into a market contract

use the `intent bidder`

```rust
/// core bidder trait
#[async_trait]
pub trait IntentBidder<N: Network> {
    type IntentProofCommitment;
    type BidParameters;
    async fn submit_bid(
        &self,
        latest_ts: u64,
        intent_id: FixedBytes<32>,
        bid_params: Self::BidParameters,
        proof_commitment: Self::IntentProofCommitment,
        signature: PrimitiveSignature,
    ) -> Result<N::ReceiptResponse>;
}
```

The client, once they determine they want to bid on a given signed intent to secure the rights to it must send a transaction to the corresponding market contract's bid() function.

The intent bidder takes in the `proof_commitment` & `signature`. Then it formats the data, build & signs a universalBombetta.bid(ProofRequest calldata request, bytes calldata signature)
ethereum transaction with their private key and lastly broadcasts that signed bid() transaction to the network thereafter, assessing the outcome of the eth transaction (success or revert).

Once the winning bid transaction is finalized into a block the client who won the auction must now track the resolution status (in the case of compute requests) or compute the described workload and the proof of compute (in the case of a compute offer). At this point the intent has a corresponding onchain intent id the market contract stored and during resolution of the intent will be submitted to the market's resolve() function to avoid slashing penalty as the client performing the compute is now obligated based on the rules of the protocol.

##### High level requirements needed to perform the compute and generate the proof for the intent

use the client's `compute worker`

```rust
/// core compute worker trait
/// run the computation needed to fulfill a compute intent's
/// computational task
#[async_trait]
pub trait ComputeWorker<I: ComputeIntent>: Send + Sync {
    async fn execute(&self, intent: &I) -> Result<WorkResult>;
}

/// Output type of a compute worker that can be used by an intent
/// resolver to resolve a compute intent.
#[derive(Debug)]
pub struct WorkResult {
    pub opaque_submission: Bytes,
    pub partial_commitment: FixedBytes<32>,
}
```

The compute providing clients perform the work described by the intent in order to proceed resolving it for a reward. This is done using their compute worker implementations, typically within the taralli-worker crate, on a system by system basis.

Once the client performing the compute has bid on a request for compute or has seen a bid placed on their offer for compute they take the intent data as input to their compute worker and then compute a proof of their result to then be formatted in the way required by the market contract's resolution schema. Then they submit it using the resolver portion of the client to receive their reward. The worker first compute's whatever is required locally and then formats it into its final form for resolution, which is the `WorkResult` type shown above. Below is a solidity function interface example of the bombetta market resolve function. The common properties among all markets will be things like the intent id and the opaqueSubmission bytes containing the proof and proof inputs that will be inputted into the verification function run on-chain.

```solidity
function resolve(bytes32 requestId, bytes calldata opaqueSubmission, bytes32 submittedPartialCommitment)
```

More details about how each compute worker takes in the compute intents as input and outputs a formatted submission to the resolve() function's of each market contract can be found in the taralli-worker crate which contains existing compute worker trait impl's.

Once the resultant proof of computation is complete and formatted by the compute worker it is then relayed to the `intent resolver` for final submission.

##### High level requirements needed to resolve the intent given the proof of computation has already been computed and formatted by a compute worker.

use the `intent resolver`

```rust
/// core resolver trait used across all compute intent markets
#[async_trait]
pub trait IntentResolver<N: Network> {
    type Intent;
    async fn resolve_intent(
        &self,
        intent_id: FixedBytes<32>,
        opaque_submission: Bytes,
    ) -> Result<N::ReceiptResponse>;
}
```

The `intent resolver` takes the intent id, and work result as input. Once the intent resolver has built the resolve() transaction with the correct inputs it is signed and broadcasted to the network (Ethereum). The intent resolver then returns the transaction's success/failure completing the life cycle of the compute intent.

#### Query (client logic)

##### High level requirements needed to search for compute intents in the server's intent db using a client

use the `intent searcher`

The client provides the following trait to query the server's query endpoint at a given system ID and filter through the list of compute intents that were returned by the query.

```rust
/// core searcher trait used across all compute intent markets
#[async_trait]
pub trait IntentSearcher {
    type Intent;
    async fn search(&self) -> Result<Self::Intent>;
}
```

As of now the query logic is very simple as the client can only query the server by system ID which returns back all of the active compute intents that have been submitted and still have an ongoing auction phase. Over time the feature set of the intent database within the server and the available query patterns the client's searcher can use will expand. This is in order to provide more refined searching to decrease the list of intents needing to be sent out of the serve and wasted resources.

##### High level requirements for the rest of the query/searcher client workflow

The process of checking intent viability when running a searcher client is the same as running a streaming client using subscibe as described above. The intents that are recieved back from the server's query endpoint are filtered through and then analyzed before bidding.

Once the searcher client has parsed through the list of available intents to find one they want to submit a bid for they use the intent bidder to do so.

After a successful bid is placed its then a matter fo tracking the intent's resolution phase to see that it resolves correctly.

### --- Taralli Worker ---

The taralli worker crate contains all the existing impl's of the client's compute worker trait in relation to the existing systems implemented within the primitives crate.

for more details, look through the risc0 compute worker module [here](../crates/taralli-worker/src/risc0/mod.rs)

### --- Taralli Binaries ---

The binaries crate contains all existing client binaries that apply to various client roles using existing systems to partcipate in the compute intent marketplace as well as the protocol server binary example.

client examples:
- requester requesting client binary [here](../bin/requesting-client/src/bin/risc0_requester.rs)
- provider streaming client binary [here](../bin/streaming-client/src/bin/risc0_provider.rs)
- requester searching client [here](../bin/searching-client/src/bin/risc0_searcher.rs)
- provider offering client [here](../bin/offering-client/src/bin/risc0_offering.rs)

server example:
- protocol server binary [here](../bin/server/src/bin/server.rs)

## ---------- Summary & Improvments ----------

The above design doc outlines the taralli protocol's 2 high level users, those looking for compute services and those looking to sell their compute. Currently, this protocol supports any use case across the programs running on the risc0 and SP1 zkVMs on sepolia tesnet, as well as a trivial arkworks groth16bn128 example. Compute providers can either compute the workloads described by the intents locally or determine if outsourcing the workloads to proving networks makes sense for them. The goal is to add in more complex use cases as example binaries as well as add in new systems to build compute intent markets around.

- The core smart contract protocol allows for programmatically defined economic agreements that apply to specific compute workloads between the 2 parties in a permission-less manor and/or a manor that accepts the underlying security assumptions of the blockchain/smart contracts it executes on (Ethereum in our case).
- The protocol server is a centralized method by which to communicate the compute intents offchain from one party to another so they can decide what to do them. (In the future, we have plans to design a way to communicate intents in a more distributed and decentralized way)
- the smart contract protocol can perhaps be made more generic such that the need for multiple market contract commitment addresses can be removed and a single address can be used making interacting with the on-chain componentes fo the protocol more straight forward...where's me solidity generics ;).
- the querying feature provided by the protocol server has lots of room for improvment to make the searcher client and compute offering workflow less of a performance over head in the case of complicated compute workloads and higher scale scenarios where upwards of thousands of active users exist.
- the implementation effort for adding new systems, validators and intent types can be decreased.
- the systems implementations should be moved out of the primitives crate in the same aspect that the compute worker impl's remain outside the client crate to allow for easier contribution and addition of systems.
- the existing modules of the client and primitives crates can be broken down into smaller crates.
