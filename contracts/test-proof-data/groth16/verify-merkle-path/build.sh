#!/bin/bash
MY_CIRCUIT= 
if [[ ! -d target ]] ; then 
    mkdir target
fi

# circom $MY_CIRCUIT.circom --r1cs --wasm --sym -o target/ -l ~/repo

cd target/${MY_CIRCUIT}_js
if [[ ! -f input.json ]] ; then 
    echo -e "{\n // Put your witness input here\n}" > input.json
    vi input.json
fi
# example witness for Multiplier2:
# {
#  "a": 3,
#  "b": 4
###  "c": 12 <-- do not include an output value (facepalm)
# }

# Generate Witness
node generate_witness.js $MY_CIRCUIT.wasm input.json witness.wtns
# Start Trusted Setup
snarkjs powersoftau new bn128 15 pot12_0000.ptau -v
# Contribute randomness
snarkjs powersoftau contribute pot12_0000.ptau pot12_0001.ptau --name="First contribution" -v
# Start generation phase
snarkjs powersoftau prepare phase2 pot12_0001.ptau pot12_final.ptau -v
# Generate a .zkey file that will contain the proving and verification keys together with all phase 2 contributions
snarkjs groth16 setup target/verify-merkle-path.r1cs pot15_final.ptau target/verify-merkle-path_0000.zkey
# Contribute to phase 2 of the ceremony:
snarkjs zkey contribute target/verify-merkle-path_0000.zkey target/verify-merkle-path_0001.zkey --name="1st Contributor Name" -v
# Export the verification key:
snarkjs zkey export verificationkey target/verify-merkle-path_0001.zkey verification_key.json

# Generate the proof 
snarkjs groth16 prove target/verify-merkle-path_0001.zkey witness.wtns proof.json public.json
# Verify the proof
snarkjs groth16 verify verification_key.json public.json proof.json
