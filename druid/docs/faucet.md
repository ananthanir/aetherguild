# faucet/faucet.go

Implements the token faucet as two `http.Handler` types registered directly on the node's HTTP server. The faucet lets users request test ETH (TETH) by entering an address and amount through a web form.

---

## Imports

### Standard Library

| Package | Purpose |
|---|---|
| `context` | Creates a 15-second timeout context for all RPC calls during a faucet request |
| `crypto/ecdsa` | The `*ecdsa.PrivateKey` type — the dev key used to sign drip transactions |
| `embed` | `//go:embed` directive to bake the compiled Svelte UI into the binary at compile time |
| `encoding/json` | `json.Marshal` for error response bodies |
| `math/big` | Arbitrary-precision integers — used to convert whole ETH amounts to wei (`amount * 10^18`) |
| `net/http` | `http.Handler` interface, request/response types, status codes |
| `strconv` | `strconv.ParseInt` to parse the `amount` form field from string to integer |
| `time` | `time.Second` for the receipt polling interval, `15 * time.Second` for the RPC timeout |

### go-ethereum Packages

| Package | Purpose |
|---|---|
| `common` | `common.Address` — 20-byte Ethereum address. `common.HexToAddress` decodes the form field into an address |
| `common/hexutil` | `hexutil.Uint64` and `hexutil.Big` — hex-encoded types for JSON-RPC responses. `hexutil.Encode` for encoding raw transaction bytes |
| `core/types` | Transaction types — `types.DynamicFeeTx` (EIP-1559), `types.SignNewTx`, `types.LatestSignerForChainID`, `types.Receipt`, `types.ReceiptStatusSuccessful` |
| `log` | Structured logger for error and warning messages |
| `rpc` | `*rpc.Client` — the in-process RPC connection to the node. `CallContext` sends JSON-RPC requests |

---

## Embedded Assets

```go
//go:embed all:dist
var assets embed.FS
```

The `dist/` directory contains the compiled Svelte faucet UI (HTML, CSS, JS). It is embedded into the Go binary at compile time by the `//go:embed` directive. The `Makefile`'s `faucet` target ensures the Svelte build (`pnpm install && pnpm run build`) runs before the Go build. At runtime, no file system access is needed to serve the UI.

---

## Types

### `UI`

```go
type UI struct{}
```

An empty struct that implements `http.Handler`. Its sole purpose is to serve the embedded static files.

### `API`

```go
type API struct {
    key     *ecdsa.PrivateKey   // dev private key for signing transactions
    rpc     *rpc.Client         // in-process RPC connection to the node
    miner   *common.Address     // dev address (source of funds, nonce owner)
    chainID *big.Int            // chain ID for EIP-155 replay protection (1337)
}
```

An `http.Handler` that processes faucet drip requests. Created once in `makeDruid` via `NewAPI`.

---

## Constructor

### `NewAPI(key, rpc, miner, chainID) *API`

Stores the four dependencies needed to sign and broadcast transactions. Called in [main.go](main.md):

```go
faucetAPI := faucet.NewAPI(privateKey, stack.Attach(), &miner, cfg.Eth.Genesis.Config.ChainID)
```

- `stack.Attach()` returns an in-process `*rpc.Client` — no TCP/HTTP overhead, direct channel to the Ethereum backend
- `chainID` comes from the genesis config, which is `1337`

---

## Response Helpers

### `respOk(w, body, ctype)`

Writes a successful response with the given content type and `X-Content-Type-Options: nosniff` to prevent MIME sniffing.

### `respErr(w, msg, code)`

Writes a JSON error response: `{"Error": "message"}` with the given HTTP status code.

---

## `UI.ServeHTTP` — Static File Handler

Registered at `/faucet/ui` and `/faucet/ui/` in `makeDruid`.

**Method check:** Only `GET` is allowed. Everything else gets `405 Method Not Allowed`.

**Routing:**

| Request path | Embedded file | Content-Type |
|---|---|---|
| `/faucet/ui/index.css` | `dist/faucet/ui/index.css` | `text/css` |
| `/faucet/ui/index.js` | `dist/faucet/ui/index.js` | `application/javascript; charset=utf-8` |
| Everything else (including `/faucet/ui` and `/faucet/ui/`) | `dist/index.html` | `text/html` |

This acts as a simple SPA server — all non-asset routes serve `index.html`, letting the Svelte app handle routing client-side.

---

## `API.ServeHTTP` — Faucet Transaction Handler

Registered at `/faucet/api` in `makeDruid`.

**Method check:** Only `POST` is accepted. Everything else gets `405 Method Not Allowed`.

### Request Format

`multipart/form-data` with two fields:

| Field | Type | Example | Description |
|---|---|---|---|
| `address` | string | `0x64...` | Recipient Ethereum address |
| `amount` | integer | `1` | Amount in whole ETH to send |

### Execution Flow

**1. Parse form**
```go
r.ParseMultipartForm(10 << 20)
```
Parses the multipart form with a 10 MB limit.

**2. Parse amount**
```go
val, err := strconv.ParseInt(amount, 10, 0)
```
Reads the `amount` field as a base-10 integer.

**3. Create timeout context**
```go
ctx, cancel := context.WithTimeout(context.Background(), 15*time.Second)
```
All subsequent RPC calls share this 15-second deadline.

**4. Fetch nonce**
```go
i.rpc.CallContext(ctx, &nonce, "eth_getTransactionCount", i.miner, "pending")
```
Gets the miner's next available nonce (including pending transactions).

**5. Fetch gas price**
```go
i.rpc.CallContext(ctx, &gasFee, "eth_gasPrice")
```
Gets the current base fee suggestion from the node.

**6. Fetch priority fee**
```go
i.rpc.CallContext(ctx, &gasTip, "eth_maxPriorityFeePerGas")
```
Gets the suggested priority tip for EIP-1559 transactions.

**7. Build and sign transaction**
```go
tx, err := types.SignNewTx(i.key, types.LatestSignerForChainID(i.chainID), &types.DynamicFeeTx{
    Nonce:     uint64(nonce),
    To:        &to,
    GasFeeCap: (*big.Int)(&gasFee),
    GasTipCap: (*big.Int)(&gasTip),
    Gas:       22000,
    Value:     new(big.Int).Mul(big.NewInt(val), new(big.Int).Exp(big.NewInt(10), big.NewInt(18), nil)),
    Data:      nil,
})
```
Creates an EIP-1559 `DynamicFeeTx`:
- `GasFeeCap` — maximum total fee per gas (base fee + tip)
- `GasTipCap` — maximum priority fee (miner tip)
- `Gas: 22000` — standard ETH transfer is 21000, plus a 1000 buffer
- `Value` — `amount * 10^18` converts whole ETH to wei
- `Data: nil` — simple value transfer, no contract interaction

Signs it with `LatestSignerForChainID(1337)` which uses EIP-1559 signing.

**8. Encode and broadcast**
```go
data, err := tx.MarshalBinary()
i.rpc.CallContext(ctx, nil, "eth_sendRawTransaction", hexutil.Encode(data))
```
Serializes the signed transaction to RLP-encoded bytes, hex-encodes them, and sends via `eth_sendRawTransaction`.

**9. Poll for receipt**
```go
for {
    time.Sleep(time.Second)
    i.rpc.CallContext(ctx, &receipt, "eth_getTransactionReceipt", tx.Hash())
    // ...check receipt.Status == ReceiptStatusSuccessful
}
```
Polls every second until the receipt arrives with `Status == 1` (success). Since the `SimulatedBeacon` produces blocks on demand, this typically completes on the first or second poll. The 15-second context provides the timeout.

**10. Respond**
```go
respOk(w, nil, "application/json")
```
Responds `200 OK` with an empty body on success.

---

## Svelte UI

The frontend (`faucet/ui/src/App.svelte`) is a simple form with:
- An address input (validated as 42 chars starting with `0x`)
- An amount input (1–100 ETH)
- A "Get TETH" button

It POSTs to the relative path `api` (resolves to `/faucet/api` when the page is at `/faucet/ui` without trailing slash, since the browser treats `/faucet/` as the base directory). On success it alerts "Credited N TETH successfully".
