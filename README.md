# Taralli Protocol

## Overview

Taralli protocol is a verifiable compute marketplace built on top of ethereum. The current prototype is described by this [design doc](./docs/taralli-design.md) with plans to maintain and update the specification of the protocol as features are added. The protocol is currently in active development and not ready for production use.

### Protocol Components

**`smart contracts`**

- [Bombetta](./contracts/src/Bombetta.sol): The generic standard for bombetta marketplace contracts to implement.
- [UniversalBombetta](./contracts/src/UniversalBombetta.sol): Implementation of the bombetta marketplace standard.

**`protocol crates`**

- [primitives](./crates/taralli-primitives/): defines shared types and functionality that is used throughout the protocol (system definitions, system IDs, common utility functions, etc).
- [protocol server](./crates/taralli-server/): rust axum api server that facilitates the communication of requests/offers for/of compute between protocol clients.
- [requester client](./crates/taralli-requester/): rust program for creating, submitting and tracking requests.
- [provider client](./crates/taralli-provider/): rust program for monitoring incoming requests from the protocol server, selecting requests, processing requests, and resolving requests.

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
RPC_URL= required for server and clients
REQUESTER_PRIVATE_KEY= required for clients
PROVIDER_PRIVATE_KEY= required for clients
RISC0_PROVER=prove
BONSAI_API_URL=https://api.bonsai.xyz/
BONSAI_API_KEY= required for using risc0 bonsai api
```
contracts/ .env
```
ETH_MAINNET_RPC_URL=
ETH_HOLESKY_RPC_URL= works with existing deploy script
ETH_LOCAL_RPC_URL=
TESTNET_PRIVATE_KEY=
LOCAL_PRIVATE_KEY=
```

NOTE: 
If you want to get the whole protocol running locally excluding the contracts/chain, the quickest/easiest way is to use holesky RPCs + the existing holesky deployment addresses [here](./contracts/deployments.json) or redeploy your own version of the contracts to holesky using existing forge script.

You can also create a local anvil fork of the holesky network and that works too.

### server config

an example server config can be found [here](./example_server_config.json). The preexisting systems the server uses are defined by the `proving_system_ids` field.

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
