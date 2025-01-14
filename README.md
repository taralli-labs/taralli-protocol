# Taralli Protocol

The current prototype hosts a simple auction on a server and allows for multiple provers to watch
for requests in the request pool and to bid in the auction

## Setup local dev environment

### environment setup

root .env
```
SERVER_PORT=3000
SERVER_URL="http://localhost:3000"
RPC_URL="http://localhost:8545"
LOG_LEVEL="info"
```
contracts/ .env
```
ETH_MAINNET_RPC_URL=
ETH_HOLESKY_RPC_URL="needed"
ETH_LOCAL_RPC_URL="needed"
TESTNET_PRIVATE_KEY=
LOCAL_PRIVATE_KEY="needed"
```

### Build

build smart contracts
```bash
cd contracts
forge build
```

build cargo workspace
```bash
cargo build
 ```

### start anvil
```bash
just start_anvil
```

### deploy contracts to anvil fork
```bash
just mock_deploy_contracts
```
```bash
just deploy_contracts
```

### update market address in server config
```bash
just update_market_addresses
```

### start taralli server (crates/taralli-server/bin)
```bash
just start_server
```

### Client example runner commands (crates/taralli-client/examples)
send create market request to server
```bash
just create_market
```

send delete market request to server
```bash
just delete_market
```

send simple proof request to server
```bash
just simple_request
```

send subscribe market request to server
```bash
just subscribe_market
```

### formatting & linting
format rust code
```bash
just format
```
lint rust code (requires changes to be staged, e.g. `git add`)
```bash
just lint
```
format smart contracts
```bash
just format_sol
```






