# Aspen

![Aspen](docs/assets/aspen.png)


**This project is in heavy development and experimentation. Expect lots of breakages, outdated, generated docs, etc.**

Hybrid Consensus Distributed Systems Framework in Rust. Built on [iroh](https://github.com/n0-computer/iroh) (P2P QUIC).

## Why

Most developer infrastructure depends on a handful of companies. Code on GitHub, builds on GitHub Actions, artifacts in someone else's registry, secrets in someone else's vault. Each piece works fine in isolation but they're all different services with different auth, different APIs, different failure modes, and you don't control any of them.

Aspen replaces that stack with one system. A cluster of nodes that talk to each other over P2P QUIC, agree on state through Raft, and store everything -- code, builds, secrets, coordination -- in a shared KV store and content-addressed blob store. Nodes find each other through gossip and DHT, punch through NATs, and work on a laptop or across continents. The whole thing runs without HTTP, DNS, or cloud accounts.

The design follows FoundationDB's layered approach: get an ordered, transactional KV store right, then build everything else on top without touching the core. A distributed lock is a KV entry with a CAS-guarded owner field. A CI pipeline is a state machine that triggers on ref updates. Git hosting is refs in the KV store and objects in the blob store. Each feature reads and writes keys. Adding one doesn't make the others more complex.

The end goal is self-hosting. Aspen's source lives in its own Forge, gets built by its own CI, and deploys to its own cluster. If it can build and ship itself reliably, it can handle other workloads too.

## Build

```bash
nix develop                    # dev shell (mold linker, all tools)
cargo build                    # core workspace
cargo nextest run              # tests
cargo nextest run -P quick     # skip slow tests
```

## Run

```bash
# Single node
cargo run --features jobs,docs,blob,hooks,automerge \
  --bin aspen-node -- --node-id 1 --cookie my-cluster

# CLI
cargo run -p aspen-cli -- kv get mykey

# 3-node local cluster
nix run .#cluster
```

## Architecture

```
Applications     Forge, CI/CD, Secrets, DNS, Automerge, FUSE
Coordination     Locks, Elections, Queues, Barriers, Semaphores, Counters
Core             KV Store (Raft) + Blob Store (iroh-blobs) + Docs (iroh-docs)
Consensus        OpenRaft (vendored) + Redb (single-fsync writes)
Transport        Iroh QUIC with ALPN multiplexing, gossip, mDNS, DHT
```

A single QUIC endpoint multiplexes Raft, client RPC, blobs, gossip, and federation via ALPN protocol tags.

## Key Design Choices

Traditional Raft does two fsyncs per write (log append, then state machine apply). Aspen does one by putting both in a single redb transaction. On top of that, concurrent writes get batched into one Raft proposal -- a configurable batch window that trades latency for throughput.

OpenRaft is vendored as a full copy in `openraft/`, not a submodule. This lets us patch consensus internals without waiting on upstream.

Git objects, CI artifacts, WASM plugin binaries, and Nix store paths all go through iroh-blobs with BLAKE3 hashing and P2P transfer. Same blob store for everything, automatic deduplication.

## Workspace

Monorepo with 82 crates under `crates/`, plus vendored OpenRaft and a few external repos (`aspen-plugins`, `aspen-wasm-plugin`, etc.).

Core traits: `ClusterController` (cluster membership) and `KeyValueStore` (distributed KV). Both implemented by `RaftNode` for production and in-memory fakes for testing.

Feature-gated -- default builds give you Raft + KV + coordination. Opt in to heavier features:

```bash
cargo build --features full          # everything
cargo build --features forge-full    # git hosting
cargo build --features ci-full       # CI/CD pipelines
```

See `Cargo.toml` `[features]` for the full list.

## Self-Hosting

```bash
nix run .#dogfood-local              # full pipeline
nix run .#dogfood-local -- start     # just the cluster
nix run .#dogfood-local -- full-loop # start -> push -> build -> deploy -> verify
```

## Verification

Pure business logic lives in `src/verified/` directories across crates. Corresponding [Verus](https://github.com/verus-lang/verus) specs in `verus/` directories prove correctness properties (fencing token monotonicity, lock mutual exclusion, overflow safety, etc.). Production code compiles normally; Verus runs separately.

```bash
nix run .#verify-verus               # verify all specs
nix run .#verify-verus coordination  # verify one crate
```

## Testing

```bash
cargo nextest run                                        # all tests
cargo nextest run -P quick                               # skip proptest/chaos/madsim
cargo nextest run -E 'test(/raft/)'                      # filter by name
nix build .#checks.x86_64-linux.kv-operations-test       # NixOS VM test
```

Deterministic simulation with madsim, property-based testing with proptest and Bolero, 46 NixOS VM integration tests under real QEMU, and FoundationDB-style buggify for fault injection.

## Docs

- [Federation](docs/FEDERATION.md)
- [Forge](docs/forge.md)
- [Plugin Development](docs/PLUGIN_DEVELOPMENT.md)
- [Deploy](docs/deploy.md)
- [Tiger Style](docs/tigerstyle.md)
- [SOPS Secrets](docs/sops.md)
- [VM Jobs](docs/VM_JOB_SUBMISSION.md)

## License

Aspen: AGPL-3.0-or-later. Vendored OpenRaft: MIT OR Apache-2.0.
