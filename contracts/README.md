## Taralli Contracts

Taralli contracts consists of:

-   **[docs](./docs)**: Specifications for Taralli contract designs (bombetta)
-   **[script](./script)**: Forge contract deployment scripts.
-   **[src](./src)**: Contract source code.
-   **[test](./test-proof-data)**: Forge test environment.
-   **[test-proof-data](./test-proof-data)**: Mock Proof data used within Taralli contract tests.

## Overview

The `UniversalBombetta` contract serves as the primary proof market contract used to facilitate auctions on, and resolution of
proof requests. The plan is to expand upon this initial generic design with proof markets centered around more specific 
protocols as well as future optimizations/UX improvements.

## Usage

### Build

```shell
$ forge build
```

### Test

```shell
$ forge test
```

### Format

```shell
$ forge fmt
```

### Deploy

```shell
$ forge script Deploy
```

### Gas Snapshots

```shell
$ forge snapshot
```