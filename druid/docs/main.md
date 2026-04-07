# main.go

The binary entry point. Responsible for wiring together every subsystem and managing the full process lifecycle.

---

## Imports

### Standard Library

| Package | Purpose |
|---|---|
| `flag` | Parses the two CLI flags: `--expose` and `--persist` |
| `io` | Provides `io.Writer` used to type-assert `os.Stderr` before wrapping with the colorable writer |
| `os` | Access to `os.Stderr`, `os.Getenv`, and `os.Signal` |
| `os/signal` | Routes OS signals (`SIGINT`, `SIGTERM`) into a Go channel for graceful shutdown |
| `syscall` | Supplies the signal constants `syscall.SIGINT` and `syscall.SIGTERM` |

### Project Packages

| Package | Purpose |
|---|---|
| `druid/faucet` | The faucet sub-package — exports `UI` (serves the Svelte app) and `NewAPI` (signs and sends ETH drip transactions). See [faucet.md](faucet.md) |

### go-ethereum Packages

| Package | Purpose |
|---|---|
| `accounts` | Wallet/account abstractions, `WalletEvent` lifecycle events, BIP-44 derivation paths (`DefaultBaseDerivationPath`, `LegacyLedgerBaseDerivationPath`) |
| `accounts/keystore` | Encrypted JSON keystore management. `LightScryptN`/`LightScryptP` are fast, low-security scrypt params appropriate for dev mode |
| `common` | Core Ethereum types — `common.Address` (20-byte address). `common.Address{0x0}` is used as the placeholder fee recipient for the simulated beacon |
| `crypto` | ECDSA/secp256k1 operations — `HexToECDSA` decodes the dev private key, `PubkeyToAddress` derives the Ethereum address |
| `eth` | Full Ethereum protocol service — `eth.New` registers EVM, tx pool, state DB, and block processor into the node |
| `eth/catalyst` | Engine API (post-merge PoS). `SimulatedBeacon` is a fake consensus layer that produces blocks on demand for dev mode |
| `eth/ethconfig` | Configuration struct for the Ethereum service. `ethconfig.Defaults` provides sane defaults for all settings |
| `eth/filters` | Event/log filtering subsystem — powers `eth_getLogs`, `eth_newFilter`, and log subscriptions |
| `eth/tracers` | Transaction execution tracers — registers `debug_traceTransaction`, `debug_traceCall`, `debug_traceBlock` RPC methods |
| `ethclient` | High-level typed Go client for Ethereum nodes. Used here for wallet `SelfDerive` and in the test suite |
| `graphql` | Registers a GraphQL endpoint on the node's HTTP server as an alternative to JSON-RPC |
| `log` | go-ethereum's structured logger — `log.Info`, `log.Warn`, `log.Error`. `SetDefault` makes it global for all go-ethereum packages |
| `node` | The p2p node / service lifecycle container — manages HTTP/WS/IPC servers, account manager, service registration, and graceful startup/shutdown |
| `rpc` | JSON-RPC layer — `rpc.API` struct registers services under namespaces. `stack.Attach()` returns an in-process `*rpc.Client` |

### Third-party Packages

| Package | Purpose |
|---|---|
| `go-colorable` | Wraps `os.Stderr` with ANSI color support on Windows where the console doesn't natively interpret escape codes |
| `go-isatty` | Detects whether stderr is a real interactive terminal or a pipe/file — decides whether to enable colored output |

---

## Global Variables

```go
var privateKey, _ = crypto.HexToECDSA("b71c71a67e1177ad4e901695e1b4b9ee17ae16c6668d313eac2f96dbcda3f291")
var miner = crypto.PubkeyToAddress(privateKey.PublicKey)
```

This is the **well-known go-ethereum dev private key** — the same key used in `geth --dev` mode and published in go-ethereum's source. It is intentionally public and never meant for production.

- `privateKey` — the decoded `*ecdsa.PrivateKey` object
- `miner` — the derived 20-byte Ethereum address, pre-funded by the genesis block, used as fee recipient and faucet source

---

## `init()`

Runs automatically before `main()`. Sets up the global structured logger.

```go
terminalOutput := io.Writer(os.Stderr)
```
Starts with plain stderr.

```go
useColor := (isatty.IsTerminal(os.Stderr.Fd()) || isatty.IsCygwinTerminal(os.Stderr.Fd())) && os.Getenv("TERM") != "dumb"
```
Checks if stderr is a real terminal (not a pipe or redirect). Cygwin check covers Windows terminal emulators. `TERM=dumb` signals no color support.

```go
if useColor {
    terminalOutput = colorable.NewColorableStderr()
}
```
On Windows, wraps stderr so ANSI codes render as actual colors.

```go
glogger := log.NewGlogHandler(log.NewTerminalHandler(terminalOutput, useColor))
log.SetDefault(log.NewLogger(glogger))
```
Creates a glog-style handler with colored or plain output, then sets it as the global default for all go-ethereum packages. The commented-out `Verbosity(0)` would silence all logs — left as a toggle.

---

## `makeDruid(expose, persist *bool) (*node.Node, *ethclient.Client)`

The core factory function. Called by `main()` and by both tests in `main_test.go` (with `expose=false, persist=false`).

### Step-by-step

**1. Build configuration**
```go
cfg := DruidConfig{
    Eth:  &ethconfig.Defaults,
    Node: DefaultNodeConfig(*expose, *persist),
}
```
`DruidConfig` is defined in [types.go](types.md). `DefaultNodeConfig` is defined in [config.go](config.md) — sets no P2P, open CORS, HTTP/WS on localhost (or `0.0.0.0` if exposed), ephemeral data (or persistent path if flagged).

**2. Create node container**
```go
stack, err := node.New(cfg.Node)
```
Creates the `node.Node` — the lifecycle/HTTP/account framework. Nothing is running yet.

**3. Import dev key into keystore**
```go
b := keystore.NewKeyStore(stack.KeyStoreDir(), keystore.LightScryptN, keystore.LightScryptP)
b.ImportECDSA(privateKey, "")
stack.AccountManager().AddBackend(b)
```
Creates a keystore with fast scrypt params, imports the dev private key with empty passphrase, and registers it as an account backend.

**4. Configure Ethereum service**
```go
SetEthConfig(stack, cfg.Eth)
```
Defined in [config.go](config.md) — sets chain ID 1337, creates the dev genesis block with the dev address pre-funded, unlocks the account, sets gas price to 1 wei.

**5. Register Ethereum backend**
```go
backend, err := eth.New(stack, cfg.Eth)
```
Registers the full Ethereum protocol — EVM, tx pool, state DB, block processor.

**6. Register tracers**
```go
stack.RegisterAPIs(tracers.APIs(backend.APIBackend))
```
Adds `debug_traceTransaction`, `debug_traceCall`, etc. to the RPC server.

**7. Create and register filter system**
```go
filterSystem := filters.NewFilterSystem(backend.APIBackend, filters.Config{
    LogCacheSize: cfg.Eth.FilterLogCacheSize,
})
stack.RegisterAPIs([]rpc.API{{
    Namespace: "eth",
    Service:   filters.NewFilterAPI(filterSystem),
}})
```
Creates the event filter engine and registers `eth_getLogs`, `eth_newFilter`, etc. under the `"eth"` RPC namespace.

**8. Register GraphQL**
```go
graphql.New(stack, backend.APIBackend, filterSystem, []string{"*"}, cfg.Node.GraphQLVirtualHosts)
```
Adds GraphQL on the HTTP server. `[]string{"*"}` allows all CORS origins.

**9. Create simulated beacon**
```go
simBeacon, err := catalyst.NewSimulatedBeacon(0, common.Address{0x0}, backend)
catalyst.RegisterSimulatedBeaconAPIs(stack, simBeacon)
stack.RegisterLifecycle(simBeacon)
```
Fake PoS consensus. Interval `0` means blocks are produced on demand when transactions arrive. Registered as a lifecycle so it starts/stops with the node.

**10. Create and register faucet**
```go
faucetAPI := faucet.NewAPI(privateKey, stack.Attach(), &miner, cfg.Eth.Genesis.Config.ChainID)
stack.RegisterHandler("Faucet UI", "/faucet/ui", faucet.UI{})
stack.RegisterHandler("Faucet UI", "/faucet/ui/", faucet.UI{})
stack.RegisterHandler("Faucet API", "/faucet/api", faucetAPI)
```
Creates the faucet with the dev key and an in-process RPC connection (`stack.Attach()`). Registers the Svelte UI and the API as HTTP handlers. Both `/faucet/ui` and `/faucet/ui/` are registered to handle trailing-slash variations.

**11. Return**
```go
return stack, ethclient.NewClient(stack.Attach())
```
Returns the configured-but-not-started node, plus an `ethclient` connected in-process. Two separate `stack.Attach()` calls create two independent in-process RPC connections.

---

## `main()`

### Flag parsing
```go
expose := flag.Bool("expose", false, "Expose the chain across the host")
persist := flag.Bool("persist", false, "Persist the chain data")
flag.Parse()
```
`make druid` passes neither flag (ephemeral, localhost). `make druid-forever` passes both (persistent, `0.0.0.0`).

### Node start
```go
stack, client := makeDruid(expose, persist)
stack.Start()
```
Assembles and starts the node — HTTP/WS/IPC servers come up, the EVM begins processing.

### Wallet goroutine
```go
events := make(chan accounts.WalletEvent, 16)
stack.AccountManager().Subscribe(events)
go func() { ... }()
```
Subscribes to wallet lifecycle events with a buffer of 16. The goroutine:

1. Opens all already-attached wallets (the keystore wallet is present at startup)
2. Listens for events in a loop:
   - **`WalletArrived`** — a new wallet is detected (e.g. USB hardware wallet plugged in), tries to open it
   - **`WalletOpened`** — wallet is ready, sets up BIP-44 derivation paths:
     - Legacy Ledger path (`m/44'/60'`) if the wallet's URL scheme is `"ledger"`
     - Standard path (`m/44'/60'/0'/0`) for all wallets
     - Calls `SelfDerive(derivationPaths, client)` to scan the chain for used accounts
   - **`WalletDropped`** — wallet disconnected, closes it

### Signal goroutine
```go
go func() {
    sigc := make(chan os.Signal, 1)
    signal.Notify(sigc, syscall.SIGINT, syscall.SIGTERM)
    <-sigc
    shutdown()
}()
```
Waits for Ctrl+C or `kill`. On first signal, `shutdown()`:

1. Calls `stack.Close()` in a sub-goroutine (non-blocking)
2. Drains up to 10 more signals, printing warnings
3. After 10 total signals the handler exits — the next signal triggers Go's default crash behavior (force-kill escape hatch)

### Block forever
```go
stack.Wait()
```
Blocks `main()` on the node's internal wait group. Released when `stack.Close()` completes.
