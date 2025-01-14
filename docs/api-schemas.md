# example schemas for taralli protocol api

this api enpoint should for example handle all groth16 proofs across all circuits and public inputs as well as

GnarkPlonkBn254,
Groth16Bn254,
SP1,
Halo2KZG,
Halo2IPA,
Risc0,

## bombetta schema

```json
{
    "proof_request_data": { /// data used to identify the nature of the proof being requested and how it should be computed.
        "proving_system_id": "UID for proving system", // for ex. `groth16Bn256`, `risc0` or `SP1`
        "circuit_uid": "UID for circuit", // ref to the circuit required for the proof request.
        "proving_system_commitment_uid": "UID for key/vmprogram", // ref to the zkey or zkVM code (elf/bytecode) required for the proof request.
        "public_inputs": [1, 1, 1] // decimal integer array of public inputs required by the rpoof request.
    },
    "onchain_proof_request": { /// witness data for the intent signature which makes up the on-chain proof request to submit a bid for.
        "proving_time": [1, 1, 1], // amount of time to resolve the proof request after bidding
        "nonce": [1, 1, 1], // nonce of the intent
        "token": [1, 1, 1], // address of the token to be awarded upon successful resolution
        "amount": [1, 1, 1], // total amount of `token` possible to receive as reward.
        "min_reward": [1, 1, 1], // minimum reward amount of `token` possible given successful resolution
        "market": [1, 1, 1], // address of the proving market contract that must be used in order to bid and resolve the proof request.
        "start_timestamp": [1, 1, 1], // start timestamp determining when bids can be placed on this proof request.
        "minimum_stake": [1, 1, 1], // minimum eth stake required to successfully bid on this proof request.
        "metadata": {
            "public_inputs_digest": [1, 1, 1], // hash of public inputs required by the proof request.
            "extra_data": [1, 1, 1], // extra data specific to the proving market contract (verification data, auction data, reward data)
        },
        "deadline": [1, 1, 1] // deadline when the signature of the proof request is no longer valid.
    },
    "signature": [1, 1], /// permit2 sig for proof request using added witness that commits to the requirements for the proof request and transfers the reward tokens to the proving market contract at bid time.
    "signer": [1,1,1] // signer address for the signature
}
```

### coordination problems

high level.

the proof requester needs to coordinate cumbersome data (circuit details, zk keys and/or vm program details) to the proof provider.
the proof provider must coordinate between the data the proof requester submits and the actual state of things in terms of the proof details making sense with the proof verification details.

### 1. the solver does not know if the extra_data of the proof request makes sense given the rest of the proof_request_data

namely...

```solidity
struct VerifierDetails {
    address verifier; // address of the verifier contract required by the requester
    bytes4 selector; // fn selector of the verifying function required by the requester
    uint256 publicInputsOffset; // offset of public inputs field within the proof submission data
    uint256 publicInputsLength; // length of public inputs field within the proof submission data
}
```

in the context of the universal bombetta market the solver must know...

- the verifier contract address submitted by the requester is in fact a verifier contract for the proving system the proof requester suggests its for.
- the function selector submitted by the requester is actually pointing to the verify function within a verifier contract.
- the public inputs offset & length described is actually where/how long the public inputs will be when verification happens.

## porchetta schema

```json
{
    "prover_intent_data": { // data about the nature of the proof the intent signer/solver is willing to provide
        "proving_system_id": "UID for proving system",
        "circuit_uid": "UID for circuit",
        "proving_system_commitment_uid": "UID for key/vmprogram",
        "public_inputs": [1,1,1,1]
    },
    "onchain_prover_intent": {  // witness data for the intent signature which makes up the on-chain prover intent to submit a bid for.
        "provingTime": [1,1,1,1], //uint256
        "nonce": : [1,1,1,1], //uint256
        "token": : [1,1,1,1], //addr
        "amount": : [1,1,1,1], //uint256
        "min_reward": [1,1,1,1], //uint256
        "market": : [1,1,1,1], //addr
        "startTimestamp": : [1,1,1,1], //uint64
        "stakeToken": : [1,1,1,1], //addr
        "stakeAmount": : [1,1,1,1], //uint256
        "metadata": {
            "public_inputs_digest": [1,1,1,1], // bytes32
            "extra_data": : [1,1,1,1], // bytes array
        },
        "deadline": [1,1,1,1], //uint256
    },
    "signature": "0xbytes" // permit2 sig for prover intent using added witness that commits to the requirements for the prover intent and transfers the collateral tokens to the proving market contract at bid time.
}
```

### coordination problems

high level the proof requester needs to be able to verify the details of how the proof will be verified in order to assert they are willing
to pay for the prover intent they are looking at within the available pool of prover intents

- the proof requester needs to know the market contract exists and is an actual market contract
- the proof requester needs to know the call to the verifying function is a valid verifying function for the purposes of the prover intent

solution #1: have taralli labs whitelist certain markets and verifier contracts, then include these in a separate filtered pool of prover intents for requesters to trust
solution #2: requesters take `market` address from off-chain data submitted to api and fetch on-chain data needed to check market/verification function correctness



```json
{
    "proof_request_data": { /// data used to identify the nature of the proof being requested and how it should be computed.
        "proving_system_id": "Groth16Bn128",
        "circuit_uid": "1",
        "proving_system_commitment_uid": "1",
        "public_inputs": [33]
    },
    "onchain_proof_request": { /// witness data for the intent signature which makes up the on-chain proof request to submit a bid for.
        "proving_time": [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 232],
        "nonce": [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        "token": [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 146, 160, 15, 196, 138, 211, 221, 74, 139, 82, 102, 168, 244, 103, 165, 42, 199, 132, 252, 131],
        "amount": [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5, 107, 199, 94, 45, 99, 16, 0, 0, 0, 0, 0],
        "min_reward": [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 10, 172, 114, 48, 72, 158, 128, 0, 0, 0, 0, 0],
        "market": [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 177, 192, 91, 73, 140, 181, 133, 104, 178, 71, 3, 105, 254, 185, 139, 0, 112, 32, 99, 218],
        "start_timestamp": [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 102, 172, 15, 19, 0, 0, 0, 0],
        "minimum_stake": [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 13, 224, 182, 179, 167, 100, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        "metadata": {
            "public_inputs_digest": [237, 147, 198, 126, 26, 155, 127, 9, 211, 180, 78, 229, 147, 54, 15, 0, 115, 96, 58, 142, 69, 65, 94, 44, 60, 105, 175, 201, 148, 161, 16, 61],
            "extra_data": [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 86, 21, 222, 183, 152, 187, 62, 77, 250, 1, 57, 223, 161,
              179, 212, 51, 204, 35, 183, 47, 67, 117, 59, 77, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0,
              0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 32]
        },
        "deadline": [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 102, 172, 19, 187, 0, 0, 0, 0]
    },
    "signature": [1, 1],
    "signer": [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 243, 159, 214, 229, 26, 173, 136, 246, 244, 206, 106, 184, 130, 114, 121, 207, 255, 185, 34, 102]
}
```

### risc0 example schema

risc0 proof request inputs in solidity
```solidity
// Metadata.extraData
UniversalBombetta.VerifierDetails memory verifierDetails = UniversalBombetta.VerifierDetails({
    verifier: address(risc0Verifier),
    selector: risc0Verifier.verify.selector,
    publicInputsOffset: 0,
    publicInputsLength: 0
});

// ProofRequest
ProofRequest memory request = ProofRequest({
    market: address(universalBombetta),
    nonce: 0,
    token: address(testToken),
    maxRewardAmount: 1000 ether, // 1000 tokens
    minRewardAmount: 0,
    minimumStake: 1 ether,
    startTimestamp: uint64(block.timestamp),
    deadline: uint64(block.timestamp + 1000),
    provingTime: 1 days,
    publicInputsDigest: keccak256(abi.encode(0)),
    extraData: abi.encode(verifierDetails)
});
```

full signed proof request sent into server
```json
{
      "proof_request_data": {
        "proving_system_id": "risc0",
        "proving_system_commitment_id": [1], // reference id to compiled elf binary of risc0 guest program
      },
      "onchain_proof_request": {
        "market": "0x0",
        "nonce": "0",
        "token": "0x0",
        "maxRewardAmount": "0x16345785d8a0000",
        "minRewardAmount": "0xb1a2bc2ec50000",
        "minimumStake": 100000000000000000,
        "startTimestamp": 1724426628,
        "deadline": 1724426688,
        "provingTime": "0x78",
        "publicInputsDigest": "0x0",
        "extraData": ""
      },
      "signature": "0x0",
      "signer": "0xf39fd6e51aad88f6f4ce6ab8827279cfffb92266"
    }
```