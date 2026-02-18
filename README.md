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

Aspen includes a plugin system for job execution backends (VM/WASM/RPC) exposed via Cargo features.

## License

AGPL-3.0-or-later
