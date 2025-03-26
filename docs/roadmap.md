# Taralli Protocol Roadmap

**`Short term goals`**
- better documentation of code, get rid of all TODO's, more testing (especially clients)
- design and build a reth-exex that replaces the server's roll in the current protocol for closer integration to L1.
- build a basic intent "solver" (compute provider/offering client that optimizes cost across multiple proof sources) for the existing systems.

**`Long term goals`**
- robust CI pipeline
- cut first alpha release of the protocol and start keeping consistent release schedule.
- deploy to testnet
- optimization (cheaper smart contracts, faster more reliable rust code)
- experiment with moving the auctions for intents onto external low latency execution layers leaving only resolution/settlement to L1.
- decentralizing the intent gossip component of the protocol from a single node to multiple.