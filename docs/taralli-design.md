# Taralli Design Specification

## ---------- Overview ----------

The taralli protocol is made up of 4 main components.

1. Bombetta Proof Marketplace contract(s)
2. Taralli Primitives
3. Taralli Server
4. Taralli Client

### --- Bombetta Proof Marketplace smart contract(s) ---

- [Bombetta specification](../contracts/docs/bombetta_spec.md)
- [universalBombetta specification](../contracts/docs/universal_bombetta_spec.md)

### --- Taralli Primitives ---

Taralli primitives contains types & utilities that are shared between the server and client.

Taralli Primitive's type `ProofRequest` describes the exact structure of the data that is submitted to the server's
submit endpoint and broadcasted through the Server Side Events (SSE) streams to corresponding subscribers who are listening 
for incoming proof requests.

```rust
/// OnchainProofRequest is the data needed to settle the request onchain
pub type OnChainProofRequest = UniversalBombetta::ProofRequest;

/// ProofRequest is the intent that's going to be gossiped by the ProofMarket
#[derive(Clone, Serialize, Deserialize)]
pub struct ProofRequest {
    pub proof_request_data: ProofRequestData,
    pub onchain_proof_request: OnChainProofRequest,
    pub signature: Signature,
    pub signer: Address,
}
```

`ProofRequest` is made up of the below types

```rust
/// Off chain data about the proof request
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ProofRequestData {
    pub proving_system_id: String,
    pub proving_system_commitment_id: String,
    pub public_inputs: Vec<u8>,
}

/// Macro expansion result UniversalBombetta::ProofRequest
#[allow(non_camel_case_types, non_snake_case)]
pub struct ProofRequest {
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

### --- Taralli Server ---

The Taralli server hosts an api server that exposes 2 endpoints...

#### Submit:

The submit endpoint allows proof requesters to submit signed proof requests, have them validated by the server, and then
gossiped from the server over SSE streams to the proof providers subscribed to listen for incoming proof requests as 
they are submitted.

##### Validation Overview:

A viable signed proof request has the following properties:
- the provided `SignedProofRequest.signature` is valid based on the provided `SignedProofRequest.signer` address & `OnChainProofRequest` data
- `SignedProofRequest.proof_request_data.proving_system_id` exists and is the same as what the server expects based on the proof market's support for that proving_system_id
- `proving_system_commitment_id` exist as reference ID
    - if the `SignedProofRequest.proof_request_data.public_inputs` field is also big enough it can require a reference ID as opposed to being put directly in the signed proof request then this same check must hold for public_inputs as well
- all `ProofRequest` fields make sense
    - `ProofRequest.market` is the real deployed universal bombetta contract we use.
    - `ProofRequest.token` is a reward token the subscriber is willing to accept along with the reward amount, (`ProofRequest.minRewardAmount` -> `ProofRequest.maxRewardAmount`)
    - `ProofRequest.startAuctionTimestamp` is already reached or going to be reached in less than 5 mins (or some other configured tolerance window)
    - `ProofRequest.endAuctionTimestamp` has not passed the current timestamp and there is time to submit a bid for the proof request (maybe also add required 2 second grace period?)
    - `ProofRequest.minimumStake` doesnt need any direct assertions made on it.
    - `ProofRequest.publicInputsCommitment` matches the hash of the public inputs in `SignedProofRequest.proof_request_data.public_inputs`
    - `ProofRequest.extraData` needs to make sense given the nature of the proof request
        - the extra_data field can be decoded into the `UniversalBombetta.VerifierDetails` struct
        - once the extra_data field is decoded, the verifier address must be checked to see it makes sense given proof request
        - for example a groth16bn128 proof request should link to a corresponding groth16 verifier contract address
        - same goes for any other proving system id such as a zkVM proof verifier contract, for ex. risc0
        - the function selector found in the VerifierDetails decoded from the extra_data field should correspond to the verifying functioning where a proof needs to be submitted for checking validity
        - further, based on the proof being verified, there will be a section/slice of the encoded calldata to this verifying function that represents the public inputs
        - the `publicInputsOffset` and `publicInputsLength` should encapsulate the public inputs of the function call to the verifier's verifying function
    - NOTE: extraData validation logic is WIP

if one of these properties inside the validation layer is checked and found not to hold then the server omits the request
and does not add it to the vlidated pool.

Overall the conditions above layout the needs/checks tp perform for the subscriber, once they do receive a new signed 
proof request over SSE, to be sure the server has already validated must if not all cases to filter out proof requests
that do not make sense.

#### Subscribe:

The Subscribe endpoint allows proof providers to subscribe to specific proving system configurations based on proving 
system IDs that they are willing to receive incoming proof request submission notifications for over SSE (Server Side Event). 
This is so the proof providers can bid on incoming proof requests seen using the server using the bombetta proof market 
contract's bid() function to secure the exclusive right to generate the proof for that proof request and then later 
resolve it using the bombetta market's resolve() function to get rewarded for fulfilling a proof request.

#### Server Technical Overview

The Taralli server is a rust axum api server that uses the taralli primitives crate along with various internal modules
to execute all the functionality encapsulated by the 2 api routes (submit and subscribe).

```rust
/// lib.rs view of the taralli server crate
pub mod app_state; // contains central state type of the server
pub mod config; // configuration for server startup
pub mod routes; // contains handler fn modules for both the submit and subscribe endpoint
pub mod subscription_manager; // contains logic around maintaining SSE streams to subscribers of various proving system IDs
pub mod utils;
pub mod validation; // contains all the generic validation logic the server performs on incoming proof requests before they are deemed "valid"
```

The axum server rust binary is [here](../crates/taralli-server/src/bin/server.rs)

### --- Taralli Client ---

The 2 different consumers (proof requester & proof provider) of the taralli protocol and bombetta proof marketplace
contract described above require various tools to facilitate the full end to end lifecycle of the proof request with ease
of integration. The Taralli client provides the tool chain the proof requester & proof provider both need in order to
participate effectively in the taralli proof marketplace.

#### Submit (client side logic)

##### High level requirements needed to submit a SignedProofRequest

use the api client

- use the proof request builder to build a signed proof request
- call the submit endpoint and receive response denoting acceptance/rejection of the submission
- upon confirmation of a submitted proof request being validated by the server and sent out to subscribers (proof providers), open up an event filter to track status of that proof request's bid
  - This stage could result in either a successful bid to move on or no successful bid after the end time of the auction timestamp has passed and then a timeout/exit state
- upon confirmation of a successful bid on the submitted proof request, open up an event filter to track the resolve event status of the proof request
  - This stage can result in a proof request that is resolved with a valid proof and the accompanying reward is rewarded to the proof provider (no slashing penalty) or the case that proof provider did not provide a valid proof in time and must be slashed their eth plus no token reward

Once the proof request has been resolved and the proof requester see this confirmed through the contract event filter, 
they can proceed using the submitted valid proof for whatever reason they needed it or if the resolve step already sent
the proof where it needed to go they are done.

##### High level requirements needed for SignedProofRequest.onchain_proof_request (includes signature and signer addr)

use the `proof request builder`

proof request builder requirements:
- ethereum private key to make signatures with, `SignedProofRequest signer & signature`
- deployed/active universal bombetta market contract address, `ProofRequest.market`
- reward token contract address, `ProofRequest.token`
- total possible token reward amount, `ProofRequest.maxRewardAmount`
- minimum possible token reward amount `ProofRequest.minRewardAmount`
- minimum eth stake collateral allowed when bidding on the proof request `ProofRequest.minimumStake`
- proving time allowed to the auction winner (provider of the proof for the request), `ProofRequest.provingTime`
- ethereum rpc provider, to fetch state about the network (`current timestamp` of latest block and `account nonce`, perhaps other stuff?)
    - used to compute a reasonable, `ProofRequest.startAuctionTimestamp` for the proof request
    - same for the endAuctionTimestamp timestamp of when the auction is over and no more bids are allowed, `ProofRequest.endAuctionTimestamp`
- the public inputs commitment to the proof they want computed `ProofRequest.publicInputsCommitment`
- extra data needed for committing to the verification data of the proof request `ProofRequest.extraData`
    - in the case of the universal bombetta market the extraData field is an abi encoded bytes array of the below information...

```solidity
/// UniversalBombetta's decoded ProofRequest.extraData type
struct VerifierDetails {
    address verifier; // address of the verifier contract required by the requester
    bytes4 selector; // fn selector of the verifying function required by the requester
    bool isShaCommitment; // bool to chose between keccak256 or sha256 for commitments, true = sha256, false = keccak256
    uint256 publicInputsOffset; // offset of public inputs field within the proof submission data (opaqueSubmission)
    uint256 publicInputsLength; // length of public inputs field within the proof submission data (opaqueSubmission)
    bool hasPartialCommitmentResultCheck; // bool representing if a proof request requires a partial commitment result check in order to be resolved
    // offset & length of the partial commitment result field within the proof submission data (opaqueSubmission)
    // that will be used to compare with the hash produced by -> keccak256(predeterminedPartialCommitment + submittedPartialCommitment)
    uint256 submittedPartialCommitmentResultOffset;
    uint256 submittedPartialCommitmentResultLength;
    // predetermined partial commitment to the submitted final commitment result of this proof request's proof submission
    // data. The proof requester commits to this hash within their signature which is used to check equivalency when
    // recomputing the partial commitment result that is contained inside the proof submission data (opaqueSubmission)
    bytes32 predeterminedPartialCommitment;
}
```

In order for the proof requester/client user to know what they need to submit in this field they need access to...
1. the bombetta's type describing the decoded bytes of ProofRequest.extraData (UniversalBombetta's VerifierDetails struct for example)
2. all the individual pieces of data inside this type (in the case of UniversalBombetta the verifier address, function selector of verifying function, etc. listed above)

Examples... 

A user wants to request a groth16bn128 proof based on circuit A, with public inputs B, so they need a verifier
contract deployed on ethereum deployed/setup for circuit A, this verifer contract will have a certain contract address,
function signature, public inputs commitment, submitted public inputs offset/length and potentially other commitments that 
need to be known by the proof requester in order for them to make the proof request that commits to those specific verifier 
details in the extraData field of the ProofRequest to make the proof request to the exact proof they need.

A user wants to request a zkVM proof (e.g. risc0 zkVM) based on zkVM program A and zkVM inputs B, so they need a zkVM verifier
contract deployed on Ethereum at a given address that can be committed to by the proof requester along with the commitments 
to the function selector, public inputs commitment if needed, submitted public inputs offset/length, potentially other
commitments that need to be known by the proof requester in order for them to make the proof request that commits to
those specific verifier details in the extraData field of the ProofRequest to make the proof request to the exact proof 
they need.

##### High level requirements needed for SignedProofRequest.ProofRequestData (NOTE: WIP, need discussion around design)

use the `proof request builder`

- `proving_system_id` obtained by some doc provided by taralli labs in github that contains all the available proof markets the server is aware of? or potentially, GET endpoint in server/website explorer for proving system markets? something else?
- `circuit_id`, `proving_system_commitment_id`, and `public_inputs`
    - all of the above fields need to be obtained by the requester first uploading their circuit or zkVM program file to a service we run that can accept storage of these circuits/zkVM programs to make them available
    - the public inputs themselves if small enough can be sent directly in the http api request, if too large they can be referenced with an ID just like the circuit id or zkVM proof id above in the public_inputs field
    - all the off-chain aspects of the proof request are communicated directly or referenced in the `SignedProofRequest.proof_request_data`
    - if the data is referenced in the signed proof request because its too large to send directly then it must be uploaded before hand then reference ids included in the proof_request_data

To upload the off-chain proof request data and generate corresponding IDs referencing each field above, we could have a 
hosted service to upload proof info (`circuit_id`, `proving_system_commitment_id`, and `public_inputs`) to...
Proof Request Data Store service (PRDS service)...name up for debate ;)

So the PRDS service would be another process where we have all the proving system id strings the taralli server is aware
of and the existing reference IDs that map to the existing set of proving system IDs 
(circuit_id, proving_system_commitment_id, etc.).

Once there is a system in place to handle referencing these fields in SignedProofRequest.ProofRequestData using IDs
the request builder simply takes these IDs and includes them in the proof request build process to submit a proof request
such that proof providers that see the proof request can then fetch the proof request data referenced by the IDs inside
SignedProofRequest.ProofRequestData using the PRDS service.

NOTE: we can start out by hardcoding a small set of fixed proofs for testing integration before completing the PRDS service

`Proof Requesters` use the PRDS to...
- upload/stream circuit data or zkVM program data in, store it and respond with an ID that allows referencing it
    - this data should be pruned once the proof request has been resolved or is stale
- potentially take in public inputs as well if they are large enough to not be able to put directly in the submit api request to the server
    - also should be pruned when possible

`Proof Providers` use the PRDS to...
- read the full proof request data using IDs in SignedProofRequest.ProofRequestData from PRDS service using IDs as lookup

#### Subscribe (client side logic)

##### High level requirements needed to subscribe to a given proof market or set of proof markets

use the api client

- need proving system ids for each market they want to subscribe to from where ever taralli labs stores them
    - proving system id registry/website? proving system ID GET endpoint?
- call the subscribe endpoint with proving system id(s) and receive an SSE stream back.
  - is SSE gud? websocket? something else?

##### High level requirements needed to check proof request viability

use the `proof request analyzer` in order to implement various checks for certain properties on incoming proof requests 
coming from an existing proof market subscription. TBD...

NOTE: the taralli server's validation layer will handle all basic checks besides those of what reward tokens will be accepted,
what minimum prices are acceptable and potentially other logic as well.

##### High level requirements needed to send bids into the universal bombetta contract

use the `proof request bidder`

The subscriber, once they determine they want to bid on a given signed proof request to secure the rights to generate
that particular proof must send a transaction to the universal bombetta contract's bid() function.

The proof request bidder takes in the `SignedProofRequest.OnChainProofRequest` &
`SignedProofRequest.signature`. Formats the data and then build/signs a universalBombetta.bid(ProofRequest calldata request)
ethereum transaction with their private key. Last step is to broadcast that signed bid() transaction and assess the outcome
of the ethereum transaction (success or revert).

Once the winning bid transaction is finalized into an ethereum block the proof provider who won the auction must now
generate the proof and submit it to the universal bombetta's resolve() function.

##### High level requirements needed to generate the proof for the SignedProofRequest

use the `proof generation inputs builder`

The subscriber/proof provider once they successfully check the viability in order to proceed bidding on a given proof request
they need all the necessary information to generate the proof. This includes items such as...

- proving system id
    - found directly in the SignedProofRequest
- circuit info OR zkVM proof info
    - circuit_id or proving_system_commitment_id reference the circuit info/zkVM program info needed to compute the proof associated to the proof request.
- public inputs
    - these are found directly in the SignedProofRequest data or referenced

NOTE: we can use the PRDS service described above in this doc to manage making this data available.

Once the proof provider has all this data using the Proof request and included IDs to fetch it they can format it 
and input it into whatever external prover binary they prefer to use in order to generate the necessary proof that 
is required in resolving the signed proof request.

This utility in the client should be used to take a proof request and convert it into an actionable execution of a
correct proof using an associated binary to run the prover with the correct inputs, once the prover outputs the
result of the proof computation it should then be relayed to the `proof request resolver` for submission.

##### High level requirements needed to resolve the proof request given the proof has already been computed

use the `proof request resolver`

The subscriber/proof provider, once they have generated the proof needed to resolve the signed proof request they bid upon,
needs the following...

- the `OnChainProofRequest` in the proof request they originally submitted a bid on using the `proof request bidder` to call universalBombetta.bid()
- the decoded `OnChainProofRequest.extraData` field which is of type `UniversalBombetta.VerifierDetails`
    - the verifier contract and verifying function outline what form the universal bombetta bytes calldata `opaqueSubmission` input looks like when calling resolve()
- the function abi of the verifier contract's verifying function that was committed to within the `OnChainProofRequest.extraData` field

```solidity
function resolve(ProofRequest calldata request, bytes calldata signature, bytes calldata opaqueSubmission) public
```

further, the `proof request resolver` should take the proof request, verifier details and function abi of the verifier's 
verifying function as input. It then constructs the opaqueSubmission bytes array which is essentially the calldata the 
proof provider wants to submit as input into the verifier contract. Once the opaqueSubmission is constructed the actual 
universalBombetta.resolve() transaction should be built using the ProofRequest the proof provider bid upon before hand 
as well as the opaqueSubmission data as defined by the resolve function signature above.

once the proof request resolver has built the resolve() transaction with the correct inputs it is signed and broadcasted.
The proof request resolver then returns the transaction's success/failure.

## ---------- Summary & Considerations ----------

The above requirements outline the taralli client's 2 core feature sets, one for the `proof requesters` and one
for the `proof providers`/subscribers.

- for the proof requester we have...
    - the `proof request builder` to help proof requester's build their desired signed proof request
    - simple api client for calling submit endpoint of server
- for the subscriber/proof provider we have...
  - the `proof request analyzer` to check if an incoming proof request which has been broadcasted to the subscriber is deemed profitable to bid upon.
  - the `proof request bidder` to send bids to Proof requests that have been deemed viable and wait for a response to the bid
    - once the bid is submitted and finalized its on the proof provider to spin up/run the necessary software to compute the correct proof (prover binary)
  - the `proof generation inputs builder` to fetch information based on the viable SignedProofRequest in order to retrieve all data necessary to generate the correct proof for it
    - this info includes proving system id, circuit and/or zkVM info and public inputs
  - the `proof request resolver` to send resolve transactions for Proof request submissions
  - simple api client to use when calling the subscribe endpoint of server???

### Client side considerations

The Proof Request Data Store (PRDS) service is not specified at all, we should discuss if this is best suited as some
service using S3 or some sort of DB we host that allows proof requesters to upload and then store circuits, zkVM programs,
and/or possibly public inputs. thereafter allowing subscribers to use the reference IDs of this data to fetch it once they need
the inputs to generate the proof.

High level, regardless of design there needs to be a way to stream especially the circuit data and/or zkVM program data
so that it can be referenced in the SignedProofRequest with the corresponding ID and later read by subscribers receiving
the SignedProofRequest over SSE when its submitted.

### Server Considerations

- user ID auth for especially subscribers is not implemented.
- the server should run its own comprehensive proof request tracking client in order to see what the status of the full protocol is (both server and smart contract state)
