# Taralli Auction

## Test

Currently we have one end-to-end test at `src/bin/server.rs` which requires:
1. the verifier server running
2. anvil running
3. the env vars `CONTRACT_ADDRESS` and `ADMIN_ADDRESS` containing addresses to the contract and its owner (see [how to deploy the contract](../taralli-ledger-client/contracts/README.md#deploy))
4. funds at the address `0x70997970C51812dc3A010C7d01b50e0d17dc79C8` (though feel free to make this configurable)

```bash
export CONTRACT_ADDRESS=
export ADMIN_ADDRESS=
# run from crates/taralli-auction
cargo test -- --nocapture
```

### Running
We can use our examples (`cargo run --bin server`, `cargo run --example simple_request`) to validate the rate limiting. Mind you, I have yet to insert the logic to direct said example's reqs to port 8081. If you don't do it, requests will flow just fine.