# Aligned Layer Test Data

The following directory contains data needed to do an aligned layer proof submission:
- test sp1 proof and associated elf file.
- aligned layer verification data, namely the raw response from submitting a proof [raw-verification-data.json](aligned_verification_data/raw-verification-data.json).
  - further I have made a node script `ProofToHex.js` to convert the decimal encoding of the raw aligned layer verification data to a hexidecimal encoding the solidity tests can consume.
  - simply run `node ProofToHex.js` after submitting an aligned layer proof and the script will take the `raw-verification-data.json` returned from aligned layer and output its hexidecimal equivalent at `verification-data.json`

## proof submission to the aligned layer testnet

example proof submission:
```bash
aligned submit \
--proving_system SP1 \
--proof ./circuits/aligned-layer/sp1_fibonacci.proof \
--proof_generator_addr 0x2B5AD5c4795c026514f8317c7a215E218DcCD6cF --vm_program ./circuits/aligned-layer/sp1_fibonacci-elf \
--aligned_verification_data_path ./circuits/aligned-layer/aligned_verification_data \
--conn wss://batcher.alignedlayer.com
```

example proof verification:
```bash
aligned verify-proof-onchain --rpc $ETH_HOLESKY_RPC_URL --chain holesky --aligned-verification-data ./circuits/aligned-layer/aligned_verification_data/raw-verification-data.json
```