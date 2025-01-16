# Taralli Protocol Roadmap

**`Short term goals`**
- better documentation of code, get rid of all TODO's, more testing (especially clients)
- add in support for orders for the existing systems (primitives, server, and clients)
- build a basic "solver" (provider client that optimizes cost across multiple proof sources) for the existing systems

**`Medium term goals`**
- design and build a reth-exex that replaces the server's roll in the current protocol and allows for more decentralized p2p communication of requests/orders across a set of many infrastructure providers as opposed to a single server host.
- robust CI pipeline
- high test coverage

**`Long term goals`**
- cut first alpha release of the protocol and start keeping consistent release schedule.
- deploy to testnet
- optimization (cheaper smart contracts, faster more reliable rust code)
- experiment with moving the auctions for request/orders onto external low latency execution layers besides Ethereum leaving only resolution/settlement to L1