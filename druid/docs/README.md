# Druid

A self-contained, single-node Ethereum development chain built on go-ethereum. Druid bundles a full EVM node, a simulated Proof-of-Stake beacon, JSON-RPC + GraphQL APIs, and a web faucet into one binary with zero external dependencies.

---

## Usage

```bash
# Run ephemeral (data wiped on exit, localhost only)
make druid

# Run persistent and exposed on the network
make druid-forever

# Run inside Docker
make druid-docker

# Run tests
make test
```

---

## File Reference

| File | Description | Docs |
|---|---|---|
| `main.go` | Binary entry point — assembles the node, manages lifecycle and shutdown | [docs/main.md](docs/main.md) |
| `types.go` | Configuration structs that group all subsystem configs | [docs/types.md](docs/types.md) |
| `config.go` | Node and Ethereum service configuration, genesis block, data directories | [docs/config.md](docs/config.md) |
| `faucet/faucet.go` | HTTP handlers for the faucet UI and token drip API | [docs/faucet.md](docs/faucet.md) |

---

## Architecture

```
druid binary
│
├── node.Node (stack)
│     ├── eth.Ethereum          — EVM, state, tx pool, block processing
│     ├── catalyst.SimBeacon    — fake PoS, produces blocks on tx arrival
│     ├── filters.FilterSystem  — eth_getLogs, eth_newFilter, subscriptions
│     ├── tracers               — debug_traceTransaction, debug_traceCall
│     ├── graphql               — GraphQL endpoint
│     └── HTTP handlers
│           ├── /faucet/ui      — embedded Svelte app (faucet.UI)
│           └── /faucet/api     — token drip endpoint (faucet.API)
│
└── ethclient.Client            — in-process RPC client (wallet derivation + tests)
```

## Ports

| Port | Protocol | Purpose |
|---|---|---|
| `8545` | HTTP | JSON-RPC (`eth`, `web3`, `net`, `debug`) + GraphQL |
| `8546` | WebSocket | JSON-RPC subscriptions |
| IPC `druid.ipc` | Unix socket | In-process / local tooling |
