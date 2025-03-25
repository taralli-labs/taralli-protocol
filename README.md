# Taralli Protocol

## Overview

Taralli protocol is a verifiable compute marketplace built on top of ethereum. The current prototype is described by this [design doc](./docs/taralli-design.md) with plans to maintain and update the specification of the protocol as features are added. The protocol is currently in active development and not ready for production use.

### Protocol Components

**`smart contracts`**

- [Bombetta](./contracts/src/abstract/Bombetta.sol): The generic standard for bombetta marketplace contracts to implement.
- [Porchetta](./contracts/src/abstract/Porchetta.sol): The generic standard for porchetta marketplace contracts to implement.
- [UniversalBombetta](./contracts/src/UniversalBombetta.sol): Implementation of the bombetta marketplace standard.
- [UniversalPorchetta](./contracts/src/UniversalPorchetta.sol): Implementation of the porchetta marketplace standard.

**`protocol crates`**

- [primitives](./crates/taralli-primitives/): shared types and functionality that is used throughout the protocol (system definitions, system IDs, common utility functions, etc).
- [server](./crates/taralli-server/): axum api server that facilitates the communication of compute intents between protocol clients.
- [client](./crates/taralli-client/): library to compose protocol clients together from their sub-components.
- [worker](./crates/taralli-worker/): library containing compute worker implementations for use in clients providing/offering compute.
- [binaries](./bin/): existing server binary as well as client binaries.

### Roadmap

A full roadmap doc is provided [here](./docs/roadmap.md) outlining what changes and additions will be made to the protocol in list of priority.

## Pre-requisites

- [rust](https://www.rust-lang.org/tools/install)
- [foundry](https://book.getfoundry.sh/getting-started/installation)

## Setup

### env

root .env
```
SERVER_URL= required for clients
API_KEY= required if you're running server with auth, otherwise not
RPC_URL= required for server and clients
REQUESTER_PRIVATE_KEY= required for clients
PROVIDER_PRIVATE_KEY= required for clients
RISC0_PROVER=prove
BONSAI_API_URL= required for using risc0 bonsai api
BONSAI_API_KEY= required for using risc0 bonsai api
SUCCINCT_PRIVATE_KEY= required for using succint network
SUCCINT_RPC_URL= required for using succint network
ENV= equivalent to the API_KEY, otherwise, default to DEVELOPMENT
```
contracts/ .env
```
ETH_MAINNET_RPC_URL=
ETH_SEPOLIA_RPC_URL=
ETH_LOCAL_RPC_URL=
LOCAL_PRIVATE_KEY=
REQUESTER_PRIVATE_KEY=
PROVIDER_PRIVATE_KEY=
```

NOTE: 
If you want to get the whole protocol running locally excluding the contracts/chain, the quickest/easiest way is to use sepolia RPCs + the existing sepolia deployment addresses [here](./contracts/deployments/sepolia_deployments.json) or redeploy your own version of the contracts to sepolia using existing forge script.

You can also create a local anvil fork of the sepolia network and that works too.

### server config

the existing server config can be found [here](./config.json). The preexisting systems the server uses are defined by the `supported_systems` field in the base validation config.

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

### Deploy Contracts
This command deploys the contracts to holesky by default but any eth rpc will work. Just be mindful that permit2 is required and certain external contracts are needed for client workflows
```bash
forge script Deploy --broadcast
```

## Run

run taralli server
```bash
cargo run --bin server
```

run provider client(s)
```bash
cargo run --example risc0_provider
```

run requester client(s)
```bash
cargo run --example risc0_requester
```

### Contributions

[here](./docs/CONTRIBUTING.md)
