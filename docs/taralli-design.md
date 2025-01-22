# Taralli Design Specification

## ---------- Overview ----------

The taralli protocol is made up of 5 main components.

1. Bombetta marketplace smart contract(s)
2. Taralli primitives
3. Taralli server
4. Taralli provider client
5. Taralli requester client

Currently many parts of the protocol are complete but in pre-alpha/alpha stage with minimal testing as well as some parts being incomplete and still a work in progress. Keep that in mind, all this software is provided as is with no guarantees. For more information on future plans read the [roadmap doc](./roadmap.md).

### --- Bombetta Marketplace smart contract(s) ---

- [Bombetta specification](../contracts/docs/bombetta_spec.md)
- [universalBombetta specification](../contracts/docs/universal_bombetta_spec.md)

### --- Taralli Primitives ---

Taralli primitives contains traits/types describing each supported system as well as types/utilities that are shared between the server and the clients.

#### Systems

All systems known by the users of the taralli protocol are organized by their respective IDs within the primitive crate's systems module. Requester clients and/or provider clients can use these types to verbosely describe the computational workload they are requesting/offering to willing buyers. Thus, allowing other parties looking at these data structures at runtime through the protocol's clients to assess their correctness/value. This in order to minimize any runtime analysis that might be needed by opposing parties. Although, system specifications through the primitives crate are not necessarily a requirement for using the protocol it is very practical and highly advised to allow the bidding party (requesters & providers) some way, before they take risk by engaging in an auction to secure the request/offer, to have as close to 100% confidence the request/offer represents what they are wlling to participate in. This goes for both requester as well as provider clients looking to buy/sell specific compute workloads.

The process to add a new system into the taralli protocol systems crate is to include the addition of the new rust module implementing the system's ProvingSystemInformation trait across a given struct which includes the desired inputs for the computational workload, the prover software that validates the compute workload was performed correctly, and the verifier constraints for where/how the proof of computation will be verified in the resolution of the request/offer onchain.

```rust
#[derive(Debug, Default)]
pub struct VerifierConstraints {
    pub verifier: Option<Address>,
    pub selector: Option<FixedBytes<4>>,
    pub is_sha_commitment: Option<bool>,
    pub public_inputs_offset: Option<U256>,
    pub public_inputs_length: Option<U256>,
    pub has_partial_commitment_result_check: Option<bool>,
    pub submitted_partial_commitment_result_offset: Option<U256>,
    pub submitted_partial_commitment_result_length: Option<U256>,
    pub predetermined_partial_commitment: Option<FixedBytes<32>>,
}

pub trait ProofConfiguration: Debug + Send + Sync + 'static {
    // return pre-determined verifier constraints of the system
    fn verifier_constraints(&self) -> VerifierConstraints;
    // validate verification configuration
    fn validate(&self, verifier_details: &VerifierDetails) -> Result<()>;
}

pub trait ProvingSystemInformation: Send + Sync + Clone + Serialize + 'static {
    type Config: ProofConfiguration;
    fn proof_configuration(&self) -> Self::Config;
    // Validate the inputs needed for proof generation
    fn validate_inputs(&self) -> Result<()>;
    // return system id based on information type
    fn proving_system_id(&self) -> ProvingSystemId;
}
```

#### Requests

Taralli Primitive's type `Request` describes the exact structure of the request data that is submitted to the server's
submit api endpoint by requester clients and broadcasted through the Server Side Events (SSE) streams to corresponding subscribers who are listening 
for incoming requests.

```rust
/// OnchainProofRequest is the data needed to settle the request onchain
pub type OnChainProofRequest = UniversalBombetta::ProofRequest;

/// Request is the data structure that's going to be gossiped by the server from requester clients that submit these 
/// requests to provider clients that would potentially fulfill them.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Request<I: ProvingSystemInformation> {
    pub proving_system_id: ProvingSystemId,
    pub proving_system_information: I,
    pub onchain_proof_request: OnChainProofRequest,
    pub signature: PrimitiveSignature,
}
```

`Request` is made up of the below types

```rust
/// ProvingSystemId is enum constructed within the taralli primitives id.rs proving_systems! macro
/// this creates an enum variant and string value that represents the system, as well as includes
/// its associated prover parameters
proving_systems! {
    (AlignedLayer, AlignedLayerProofParams, "aligned-layer"),
    (Arkworks, ArkworksProofParams, "arkworks"),
    (Gnark, GnarkProofParams, "gnark"),
    (Risc0, Risc0ProofParams, "risc0"),
    (Sp1, Sp1ProofParams, "sp1")
    // insert new system(s) here
}

/// ProvingSystemInformation is the trait impl'd over each systems' input parameter struct
/// describing its expected structure, validation logic and also the verifier constraints
/// for how the compute workload will be verified
pub trait ProvingSystemInformation: Send + Sync + Clone + Serialize + 'static {
    fn validate_prover_inputs(&self) -> Result<()>;
    fn verifier_constraints() -> VerifierConstraints;
}

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

#### Offers

The support for requests is detailed above. The support for offers is a WIP and will be added soon along with all necessary high level logic and type information. Reference the roadmap doc for more info.

### --- Taralli Server ---

The Taralli server hosts an api server that exposes 2 endpoints...

#### Submit:

The submit endpoint allows requesters to submit signed requests, have them pre-validated by the server, and then
gossiped from the server over SSE streams to the providers subscribed to listen for incoming requests as 
they are submitted.

NOTE: In the future submit will also be used to handle order submissions by providers.

##### Validation Overview:

NOTE: some of these validation criteria are currently performed/not performed by the taralli protocol server before gossiping 
the requests/offers to the market participants. This feature of the server validating submissions is open to interpretation 
and will change overtime as to find a good middle ground between allowing clients to trust the server to save them some time 
having to re-validate common attributes of submissions while at the same time not limiting the server's throughput by loading 
it up with many complex validation tasks before simply gossiping the data to clients and allowing clients to do their own validation.

A viable request has the following properties:
- the `proving_system_id` exists
- the `proving_system_information` exists and corresponds to the correct proving system id
- all `onchain_proof_request` fields make sense given the accompanying request data
    - `market` is the real deployed universal bombetta contract we use.
    - `nonce` has not already been used (WIP)
    - `token` is a reward token the subscriber is willing to accept along with the reward amount, (`minRewardAmount` -> `maxRewardAmount`)
    - `startAuctionTimestamp` is already reached or going to be reached in less than 5 mins (or some other configured tolerance window)
    - `endAuctionTimestamp` has not passed the current timestamp and there is still time to submit a bid for the request.
    - `proving time` is a long enough duration given computational complexity of the given compute workload & proof.
    - `minimumStake` doesnt need any direct assertions made on it as of now (up to the clients to decide if this is ok).
    - `publicInputsCommitment` matches the hash of the public inputs in `proving_system_information` if there are public inputs
    - `extraData` needs to make sense given the nature of the request. Referencing the [UniversalBombetta smart contract spec](../contracts/docs/universal_bombetta_spec.md) this field is generic but currently being used to describe the verifier constraints of a given proof provided for the request.
        - the extra_data field can be decoded into the solidity type, `UniversalBombetta.VerifierDetails`
        - the decoded verifier details can be checked against the system's corresponding verifier constraints type
        - beyond the verifier constraints, the dynamic inputs to the system's prover for a specific request can checked.
            - for example a groth16bn128 based request should link to a corresponding groth16 verifier contract address for the correct circuit.
        - the same goes for inputs/public inputs and other dynamic fields that will be checked on a request by request basis.
    - NOTE: extraData validation logic is WIP and needs improvement.
- the provided `signature` is valid based on the provided `signer` address & `OnChainProofRequest` data


if one of these properties inside the validation layer of the server is checked and found not to hold then the server omits the request
and does not add it to the validated pool that will be sent out to those listening for new submissions.

Overall the conditions above attempt to comprehensively layout the possible and common needs/checks to perform for the provider client, once they do receive a new submission over SSE, to be sure the server has already validated most if not all cases to filter out requests that do not make sense to look at bidding/taking economic risk on.

NOTE: With future support of offers a lot of the same logic will exist just modified to adhere to offer types and oriented around validating for potential buyers of compute offerings as opposed to buyers of compute requests.

#### Subscribe:

The Subscribe endpoint allows provider clients to subscribe to specific proving system configurations based on proving 
system IDs that they are willing to receive incoming request submission notifications for over SSE. This is so the 
provider clients can bid on incoming requests seen from the protocol server there by using the market contract's bid() 
function to secure the exclusive right to perform the compute and generate the proof for that request within the agreed upon 
deadline. Then later resolve the request using the market's resolve() function to get rewarded for correctly fulfilling a request.

#### Viewing Offers

A method by which requesting parties looking for specific compute services can view real time available offers being submitted is a work in progress 
along with the rest of the implementation detail of protocol support for offers. Read the roadmap doc for more info.

#### Server Technical Overview

The Taralli server is a rust axum api server that uses the taralli primitives crate along with various internal modules
to execute all the functionality encapsulated by the 2 api routes (submit and subscribe).

```rust
/// lib.rs view of the taralli server crate
pub mod app_state; // contains central state type of the server
pub mod config; // configuration for server startup
pub mod error; // server specific errors
pub mod routes; // contains handler fn modules for both the submit and subscribe endpoint
pub mod subscription_manager; // contains logic around maintaining SSE streams to subscribers of various proving system IDs
pub mod validation; // contains all the generic validation logic the server performs on incoming requests before they are deemed "valid"
```

An example axum server rust binary is [here](../crates/taralli-server/src/bin/server.rs)

### --- Taralli Clients (requester & provider) ---

The 2 different consumer types of the taralli protocol described above require various tools to facilitate the full end to end lifecycle of the 
request/order process with ease of integration/interactivity. The provider and requester client crates provide the tool chain that both parties 
need in order to participate effectively in the taralli marketplace. The abstract goal of the protocol is to coordinate those who need compute
with those who have compute together such that the price of the desired compute workloads is discovered as fast and as accurately as possible.

#### Submit (client logic)

##### High level requirements needed to submit a Request

requester client:
- use the request builder to build a request, then sign it with an eth account.
- call the submit endpoint of the taralli server and receive a response denoting acceptance/rejection of the submission.
- upon confirmation of a submitted request being validated by the server and sent out to subscribers (provider clients), open up an event filter to track the status of that request's bid() in the market contract.
  - This stage could result in either a successful bid event to move on to the resolve phase or no successful bid after the end time of the auction timestamp has passed and then a timeout/exit state.
- Upon confirmation of a successful bid on the submitted request, the client opens up an event filter to track the resolve() event in the market contract to see if the request was resolved by the selected provider.
  - This stage in the client's execution can result in a request that is resolved with a valid proof of the computation and the accompanying reward is given by the requester's eth account to the provider (no slashing penalty) or the case that provider did not provide a valid proof of the requested compute and must be slashed their eth stake plus no token reward (slashing can happen from an invalid proof or deadline timestamp being reached).

Once the request has been resolved and the requester see this confirmed through the market contract event filter, they can proceed using the onchain
submitted valid proof for whatever reason they needed it for or if the resolve step already involved sending the proof where it needed to go they are done.

##### Requirements needed for onchain_proof_request (includes signature and signer addr)

use the `request builder`

request builder requirements:
- ethereum account to make signatures with, `signer & signature`
- deployed/active universal bombetta market contract address, `market`
- reward token contract address, `token`
- total possible token reward amount, `maxRewardAmount`
- minimum possible token reward amount `minRewardAmount`
- minimum eth stake collateral allowed when bidding on the request `minimumStake`
- proving time allowed to the auction winner (provider for the request), `provingTime`
- ethereum rpc provider, to fetch state about the network (`current timestamp` of latest block and `account nonce`, perhaps other stuff?)
    - used to compute a reasonable, `startAuctionTimestamp` for the request
    - same for the `endAuctionTimestamp` when the auction is over and no more bids are allowed
- the public inputs commitment to the proof they want computed `publicInputsCommitment`
- extra data needed for committing to the verification data of the request in the case of the universal bombetta market, `extraData`
    - below is the structure encoded into the extra data field

```solidity
/// UniversalBombetta's decoded extraData type
struct VerifierDetails {
    address verifier; // address of the verifier contract required by the requester
    bytes4 selector; // fn selector of the verifying function required by the requester
    bool isShaCommitment; // bool to chose between keccak256 or sha256 for commitments, true = sha256, false = keccak256
    uint256 publicInputsOffset; // offset of public inputs field within the proof submission data (opaqueSubmission)
    uint256 publicInputsLength; // length of public inputs field within the proof submission data (opaqueSubmission)
    bool hasPartialCommitmentResultCheck; // bool representing if a request requires a partial commitment result check in order to be resolved
    // offset & length of the partial commitment result field within the proof submission data (opaqueSubmission)
    // that will be used to compare with the hash produced by -> keccak256(predeterminedPartialCommitment + submittedPartialCommitment)
    uint256 submittedPartialCommitmentResultOffset;
    uint256 submittedPartialCommitmentResultLength;
    // predetermined partial commitment to the submitted final commitment result of this request's proof submission
    // data. The requester commits to this hash within their signature which is used to check equivalency when
    // recomputing the partial commitment result that is contained inside the proof submission data (opaqueSubmission)
    bytes32 predeterminedPartialCommitment;
}
```

In order for the requester to know what they need to submit in this field they need access to...
1. the bombetta's type describing the decoded bytes of extraData (UniversalBombetta's VerifierDetails struct for example)
2. all the individual pieces of data inside this encoded type (in the case of UniversalBombetta the verifier address, function selector of verifying function, etc. listed above)

Examples... 

A user wants to request a groth16bn128 proof based on circuit A, with public inputs B, so they need some verifier
contract deployed on ethereum deployed/setup for circuit A, this verifer contract will have a certain contract address,
function signature, public inputs commitment, submitted public inputs offset/length and potentially other commitments that 
need to be known by the requester in order for them to make the request that commits to those specific verifier 
details in the extraData field of the OnChainProofRequest to make the request to the exact proof they need.

A user wants to request a zkVM proof (e.g. risc0 zkVM) based on zkVM program A and zkVM inputs B, so they need a zkVM verifier
contract deployed on Ethereum at a given address that can be committed to by the requester along with the commitments 
to the function selector, public inputs commitment if needed, submitted public inputs offset/length, potentially other
commitments that need to be known by the requester in order for them to make the request that commits to those specific verifier 
details in the extraData field of the OnChainProofRequest to make the request to the exact proof they need.

##### Requirements needed for the ProvingSystemInformation field of the request

use the `request builder`

- `proving_system_id` obtained by taralli primitives crate system definitions
- `proving_system_information` also obtained by taralli primitives crate system definitions

Once the proving system information type has been laid out, the requester fills in the necessary data for this struct using whatever tools necessary given their
understanding of the compute request they are making to give the provider the necessary information they need to perform the compute task and prove it in the process.

#### Subscribe (client logic)

##### High level requirements needed to subscribe to a given market or set of markets

use the provider client

- using proving system id(s) for each market from taralli primitives as input, call the subscribe endpoint of the server and receive an SSE stream back.

##### High level requirements needed to check request viability

use the `request analyzer` in order to use/implement various checks for certain properties on incoming requests 
coming from an existing proof market subscription. TBD...

NOTE: the taralli server's validation layer will handle some basic checks besides more abstract qualities that should be decided by the clients,
such as what reward tokens and minimum prices are acceptable and potentially other logic as well.

##### High level requirements needed to send bids into the universal bombetta contract

use the `request bidder`

The subscriber, once they determine they want to bid on a given signed request to secure the rights to generate
that particular compute and proof for it, must send a transaction to the universal bombetta contract's bid() function.

The request bidder takes in the `OnChainProofRequest` & `signature`. Then it formats the data, build/signs a universalBombetta.bid(ProofRequest calldata request, bytes calldata signature)
ethereum transaction with their private key and lastly broadcast that signed bid() transaction to the network thereafter, assessing the outcome of the eth transaction (success or revert).

Once the winning bid transaction is finalized into a block the provider who won the auction must now generate the proof of compute which now has a corresponding onchain `requestId` the contract stores and submit it to the universal bombetta's resolve() function
to avoid being slashed as they are now obligated based on the rules of the protocol.

##### High level requirements needed to generate the proof for the Request

use the provider client's `compute worker`

The subscriber/provider once they successfully check the viability in order to proceed bidding on a given request
they need all the necessary information to perform the computation and generate the proof. This info can be founded all in the
Request information.

Once the provider has all this data using the request and included IDs to fetch it they can format it 
and input it into whatever compute worker program they prefer as long as it adheres to the requests guidlines to use for to generating the 
necessary proof that is required in resolving the signed request.

This utility in the client is used to take a request and convert it into an actionable execution input of a
correct proof using an associated binary to run the computation and prover with the correct inputs. Once the prover outputs the
resultant proof it should then be formatted into its final form for resolution.

The subscriber/provider, once they have perfomed the compute and generated the proof needed using the compute worker for the request they bid upon needs the following...
- the decoded structure of the `extraData` field which is of type `UniversalBombetta.VerifierDetails` in the protocol's current most common case.
    - the verifier contract and verifying function's definintion to know what form the universal bombetta encoded bytes calldata `opaqueSubmission` input looks like when calling resolve()
- the function abi of the verifier contract's verifying function that was committed to within the `extraData` field.

```solidity
function resolve(bytes32 requestId, bytes calldata opaqueSubmission, bytes32 submittedPartialCommitment)
```

More details about how each compute worker takes in the Request as input and outputs a formatted submission to the resolve() function of the market can be found in the 
provider client's compute worker trait impl's.

Once the resultant proof of computation is complete and formatted by the compute worker it is then relayed to the `request resolver` for final submission.

##### High level requirements needed to resolve the request given the proof of computation has already been computed and formatted by a worker.

use the `request resolver`

The `request resolver` takes the requestId, and work result as input. Once the request resolver has built the resolve() transaction with the 
correct inputs it is signed and broadcasted to the network. The request resolver then returns the transaction's success/failure.

## ---------- Summary & Considerations ----------

The above design doc outlines the taralli protocol's 2 core users, the `requesters` who are looking at this market protocol for potential compute services they would buy 
and `providers` who are looking to sell compute through this marketplace.

- The core smart contract protocol (bombetta market and potentially future market contracts) allows for programmatically defined economic agreements over compute workloads between the 2 parties in a permission-less manor and/or a manor that accepts the underlying security assumptions of the blockchain/smart contracts it executes on.
- The protocol server which is a simple centralized method by which to communicate the requests/offers offchain from one party to another so they can decide what to do them. (In the future we have plans to remove the server as a central point of failure/trust and will replace it with something more decentralized/permission-less)
- The requester client which allows those seeking compute to ask compute providers to fulfill their requests at a given economic cost.
- The provider client which allows those with compute to fulfill requests from requesters and/or make offers to requesters for specific compute workloads they are willing to run for a certain economic cost.
