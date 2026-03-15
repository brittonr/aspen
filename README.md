# Aspen

![Aspen](docs/assets/aspen.png)


**This project is in heavy development and experimentation. Expect lots of breakages, outdated, generated docs, etc.**

Hybrid Consensus Distributed Systems Framework in Rust. Built on [iroh](https://github.com/n0-computer/iroh) (P2P QUIC).

## Why

| | Typical stack | Aspen |
|---|---|---|
| Code | GitHub | Forge |
| Builds | GitHub Actions | Built-in CI |
| Artifacts | External registry | iroh-blobs (BLAKE3, P2P) |
| Secrets | Vault | SOPS in KV store |
| Auth | Per-service | Cluster identity |
| Transport | HTTP + DNS | P2P QUIC |
| Discovery | DNS, load balancers | Gossip, mDNS, DHT |

Aspen builds and hosts itself: source in its own Forge, built by its own CI, deployed to its own cluster.

## Architecture

```
Applications     Forge, CI/CD, Secrets, DNS, Automerge, FUSE
Coordination     Locks, Elections, Queues, Barriers, Semaphores, Counters
Core             KV Store (Raft) + Blob Store (iroh-blobs) + Docs (iroh-docs)
Consensus        OpenRaft (vendored) + Redb (single-fsync writes)
Transport        Iroh QUIC (ALPN multiplexing, gossip, mDNS, DHT)
```

FoundationDB-style layered design: ordered transactional KV at the bottom, everything else built on top as key reads and writes.

| Decision | Detail |
|---|---|
| Single fsync | Log + state machine apply in one redb transaction |
| Write batching | Concurrent writes batched into one Raft proposal |
| Vendored consensus | Full OpenRaft copy, patchable without upstream |
| Content-addressed storage | Git objects, CI artifacts, WASM, Nix paths through iroh-blobs |
| ALPN multiplexing | One QUIC endpoint for Raft, RPC, blobs, gossip, federation |

Core traits: `ClusterController` (membership) and `KeyValueStore` (KV ops), implemented by `RaftNode`.

## Usage

```bash
nix develop                    # dev shell
cargo build                    # build
cargo build --features full    # everything
```

```bash
cargo run --features jobs,docs,blob,hooks,automerge \
  --bin aspen-node -- --node-id 1 --cookie my-cluster

cargo run -p aspen-cli -- kv get mykey

nix run .#cluster              # 3-node local cluster
```

Self-hosting:

```bash
nix run .#dogfood-local              # full pipeline
nix run .#dogfood-local -- full-loop # start -> push -> build -> deploy -> verify
```

## Testing and Verification

```bash
cargo nextest run                                    # all tests
cargo nextest run -P quick                           # skip slow tests
cargo nextest run -E 'test(/raft/)'                  # filter
nix build .#checks.x86_64-linux.kv-operations-test   # NixOS VM test
nix run .#verify-verus                               # Verus proofs
nix run .#verify-verus coordination                  # single crate
```

| Approach | Tool | |
|---|---|---|
| Deterministic simulation | madsim | Reproducible distributed scenarios |
| Property-based testing | proptest, Bolero | Edge cases, fuzzing |
| VM integration | NixOS + QEMU | Full cluster tests |
| Fault injection | buggify | FoundationDB-style chaos |
| Formal verification | [Verus](https://github.com/verus-lang/verus) | Proofs in `verus/`, code in `src/verified/` |

## Docs

- [Deploy](docs/deploy.md)
- [Federation](docs/FEDERATION.md)
- [Forge](docs/forge.md)
- [Host ABI](docs/HOST_ABI.md)
- [Identity Persistence](docs/identity-persistence.md)
- [KV Branching](docs/kv-branching.md)
- [Plugin Development](docs/PLUGIN_DEVELOPMENT.md)
- [SOPS Secrets](docs/sops.md)
- [Tiger Style](docs/tigerstyle.md)
- [VM Jobs](docs/VM_JOB_SUBMISSION.md)

## License

Aspen: AGPL-3.0-or-later. Vendored OpenRaft: MIT OR Apache-2.0.
