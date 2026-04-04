# types.go

Defines the configuration structs that group all subsystem configs into a single value passed through `makeDruid`.

---

## Imports

| Package | Purpose |
|---|---|
| `eth/ethconfig` | Provides `ethconfig.Config` — the Ethereum service configuration (genesis, gas limits, sync mode, tx pool, etc.) |
| `metrics` | Provides `metrics.Config` — Prometheus/metrics configuration |
| `node` | Provides `node.Config` — the node container configuration (HTTP/WS/IPC, P2P, data directory, account manager, etc.) |

---

## Types

### `ethstatsConfig`

```go
type ethstatsConfig struct {
    URL string `toml:",omitempty"`
}
```

A minimal struct with a single field — the URL for an [ethstats](https://ethstats.net/) reporting server. The `toml:",omitempty"` tag means the field is omitted from TOML output when empty.

> Currently unused — `DruidConfig.Ethstats` is never populated in `makeDruid`. Exists as an extension point.

---

### `DruidConfig`

```go
type DruidConfig struct {
    Eth      *ethconfig.Config
    Node     *node.Config
    Ethstats *ethstatsConfig
    Metrics  *metrics.Config
}
```

The top-level configuration container. Created at the start of `makeDruid` in [main.go](main.md):

```go
cfg := DruidConfig{
    Eth:  &ethconfig.Defaults,
    Node: DefaultNodeConfig(*expose, *persist),
}
```

| Field | Type | Populated? | Purpose |
|---|---|---|---|
| `Eth` | `*ethconfig.Config` | Yes | Set to `&ethconfig.Defaults`, then mutated by `SetEthConfig` in [config.go](config.md) — sets chain ID 1337, dev genesis, gas price 1 wei |
| `Node` | `*node.Config` | Yes | Built by `DefaultNodeConfig` in [config.go](config.md) — no P2P, open CORS, HTTP/WS hosts based on `--expose`, data dir based on `--persist` |
| `Ethstats` | `*ethstatsConfig` | No | Reserved for future ethstats integration |
| `Metrics` | `*metrics.Config` | No | Reserved for future Prometheus/metrics integration |

---

## How it flows

```
main.go: makeDruid()
    │
    ├── cfg.Node = DefaultNodeConfig(expose, persist)   ← config.go
    ├── cfg.Eth  = &ethconfig.Defaults
    │
    ├── node.New(cfg.Node)                              ← creates the node container
    ├── SetEthConfig(stack, cfg.Eth)                    ← config.go mutates cfg.Eth
    └── eth.New(stack, cfg.Eth)                         ← uses the fully configured cfg.Eth
```
