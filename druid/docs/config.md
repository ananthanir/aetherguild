# config.go

Contains all configuration logic for the node and Ethereum service. Defines no types — only functions. This file is where chain identity (network ID, genesis), node networking (P2P, HTTP, WS), and data persistence are all decided.

---

## Imports

### Standard Library

| Package | Purpose |
|---|---|
| `math/big` | Arbitrary-precision integers — used to set `GasPrice = big.NewInt(1)` (1 wei) |
| `os` | File system access (`os.Open`, `os.Getenv`) for data directory helpers |
| `os/user` | `user.Current()` to resolve the home directory when `$HOME` is unset |
| `path/filepath` | OS-aware path joining for data directories |
| `runtime` | `runtime.GOOS` to branch on macOS / Windows / Linux for data directory paths |

### go-ethereum Packages

| Package | Purpose |
|---|---|
| `accounts` | Provides `accounts.Account` — used to track the developer account |
| `accounts/keystore` | `KeyStoreType` constant to look up the keystore backend, `*KeyStore` to unlock accounts |
| `common` | `common.Address{}` — the zero address used to check if `PendingFeeRecipient` is unset |
| `core` | `core.DeveloperGenesisBlock(gasCeil, &address)` — creates a genesis block with the dev address pre-funded with a huge ETH balance and all Ethereum precompile contracts deployed |
| `eth/ethconfig` | `ethconfig.Config` struct that this file mutates, and `ethconfig.Defaults.Miner.GasCeil` for the gas ceiling |
| `log` | Structured logger for `log.Info` and `log.Error` messages |
| `node` | `node.DefaultConfig`, `node.Config`, `node.DefaultHTTPHost`, `node.DefaultWSHost` — the base config and host constants |

---

## Functions

### `DefaultNodeConfig(expose, persist bool) *node.Config`

Returns a `*node.Config` customized for a solo dev node. Starts from `node.DefaultConfig` and overrides:

| Setting | Value | Why |
|---|---|---|
| `Name` | `"druid"` | Node identity shown in `web3_clientVersion` |
| `DataDir` | `""` | Empty string means go-ethereum uses a temp directory (ephemeral) |
| `IPCPath` | `"druid.ipc"` | Named IPC socket for local tool connections |
| `P2P.MaxPeers` | `0` | No peer connections — solo node |
| `P2P.ListenAddr` | `""` | Don't listen for incoming P2P connections |
| `P2P.NoDial` | `true` | Never attempt outbound P2P connections |
| `P2P.NoDiscovery` | `true` | Disable peer discovery (DHT) |
| `P2P.DiscoveryV5` | `false` | Disable discv5 protocol |
| `UseLightweightKDF` | `true` | Use fast scrypt params for keystore — appropriate for dev, not production |
| `HTTPHost` | `node.DefaultHTTPHost` (`"localhost"`) | Bind HTTP to localhost |
| `HTTPCors` | `["*"]` | Allow all CORS origins — necessary for browser dev tools and dApps |
| `HTTPModules` | `["eth", "web3", "net"]` | RPC namespaces exposed over HTTP |
| `WSHost` | `node.DefaultWSHost` (`"localhost"`) | Bind WebSocket to localhost |

**Flag overrides:**

- If `expose == true`: both `HTTPHost` and `WSHost` change to `"0.0.0.0"` — accessible from any network interface. Used by `make druid-forever` and Docker.
- If `persist == true`: `DataDir` is set to `DefaultDataDir() + "/druid"` — chain data survives restarts.

---

### `SetEthConfig(stack *node.Node, cfg *ethconfig.Config)`

Mutates the `*ethconfig.Config` in place to configure the Ethereum service. Called after the node container exists and the keystore has been imported.

**Step-by-step:**

**1. Basic settings**
```go
cfg.NetworkId = 1337
cfg.SyncMode = ethconfig.FullSync
cfg.EnablePreimageRecording = true
```
- `1337` is the conventional dev chain ID
- `FullSync` processes all blocks (no snap/light sync — there's only one node)
- Preimage recording stores the preimages of hashed keys, needed by some debug/tracing tools

**2. Retrieve keystore**
```go
var ks *keystore.KeyStore
if keystores := stack.AccountManager().Backends(keystore.KeyStoreType); len(keystores) > 0 {
    ks = keystores[0].(*keystore.KeyStore)
}
```
Looks up the first keystore backend from the account manager. This is the keystore created in `makeDruid` that already contains the imported dev key.

**3. Resolve developer account**
```go
if cfg.Miner.PendingFeeRecipient != (common.Address{}) {
    developer = accounts.Account{Address: cfg.Miner.PendingFeeRecipient}
} else if accs := ks.Accounts(); len(accs) > 0 {
    developer = ks.Accounts()[0]
} else {
    developer, err = ks.NewAccount(passphrase)
}
```
Three-tier fallback:
1. Use `PendingFeeRecipient` if already set (it isn't in Druid's case)
2. Use the first account in the keystore (this is the imported dev key — the path Druid takes)
3. Create a brand new account as last resort

**4. Set fee recipient and unlock**
```go
cfg.Miner.PendingFeeRecipient = developer.Address
ks.Unlock(developer, passphrase)
```
The miner needs a fee recipient or it refuses to start. The account is unlocked with the empty passphrase so it can sign blocks.

**5. Create genesis block**
```go
cfg.Genesis = core.DeveloperGenesisBlock(ethconfig.Defaults.Miner.GasCeil, &developer.Address)
```
`DeveloperGenesisBlock` creates a genesis with:
- The developer address funded with a massive ETH balance
- All Ethereum precompile contracts (ecrecover, sha256, etc.) pre-deployed
- Shanghai/Cancun-era chain config enabled from block 0

**6. Set gas price**
```go
cfg.Miner.GasPrice = big.NewInt(1)
```
1 wei gas price — transactions are essentially free.

---

### `DefaultDataDir() string`

Returns the OS-appropriate path for persistent chain data. Only used when `--persist` is passed.

| OS | Path |
|---|---|
| macOS (`darwin`) | `~/Library/AetherGuild` |
| Windows | `%LOCALAPPDATA%\AetherGuild` (with fallback to `%APPDATA%\Roaming\AetherGuild` if that already exists) |
| Linux / other | `~/.aetherguild` |

---

### Helper Functions

**`windowsAppData() string`**

Returns `%LOCALAPPDATA%`. Panics if the variable is undefined (Windows XP is not supported).

**`isNonEmptyDir(dir string) bool`**

Returns true if `dir` exists and contains at least one entry. Used on Windows to check if the legacy `Roaming\AetherGuild` path already has data — if so, it's used instead of `LOCALAPPDATA` to avoid breaking existing setups.

**`homeDir() string`**

Returns the user's home directory. Checks `$HOME` first (works on Linux/macOS and in most CI environments), then falls back to `user.Current().HomeDir`.
