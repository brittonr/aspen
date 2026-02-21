# Aspen

Aspen is a hybrid-consensus distributed systems framework in Rust, built on top of [Iroh](https://github.com/n0-computer/iroh): local-first Raft for strong coordination with eventual, peer-to-peer convergence for global state. It also supports federated cluster-to-cluster replication. The codebase is under active development.

## Build

```bash
# Optional: enter the Nix dev shell
nix develop

cargo build
```

## Run

```bash
# Node binary (requires features)
cargo run -p aspen --bin aspen-node --features "jobs,docs,blob,hooks" -- --node-id 1 --cookie dev

# CLI and TUI
cargo run -p aspen-cli -- --help
cargo run -p aspen-tui -- --help
```

## Features

Most functionality is behind Cargo features. See `Cargo.toml` for available flags.

## Core KV

Aspen ships a core key-value store backed by Raft for strongly consistent reads/writes within a cluster, then converges via peer-to-peer replication across clusters.

## Plugins

Aspen includes a WASM plugin system for extending clusters with custom request handlers.
Plugins run sandboxed in hyperlight-wasm with capability-based permissions, KV namespace
isolation, and Ed25519 signing.

See [Plugin Development Guide](docs/PLUGIN_DEVELOPMENT.md) for building your own plugins.

## Federation

Independent Aspen clusters can discover each other, share content, and synchronize
resources across organizational boundaries — without HTTP, DNS, or any central
authority. Federation is built on Ed25519 cluster identities, DHT-based discovery
(BitTorrent Mainline BEP-44), rate-limited gossip, and a QUIC-based sync protocol
with three-layer cryptographic verification.

See [Federation Guide](docs/FEDERATION.md) for the full architecture and API reference.

## License

AGPL-3.0-or-later
