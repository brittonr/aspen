# Aspen

![Aspen](docs/assets/aspen.png)

**Heavy development. Expect breakages and outdated docs.**

Distributed systems primitives in Rust, built on [iroh](https://github.com/n0-computer/iroh) P2P QUIC. Ordered transactional KV at the bottom (Raft consensus), everything else built on top as key reads and writes.

Aspen's source lives in its own Git forge, built by its own CI, deployed to its own cluster. `nix run .#dogfood-local -- full` runs this end-to-end, writes an operator-visible receipt, and publishes the final success receipt into Aspen KV before cleanup; inspect local evidence later with `receipts list/show`, or use `full --leave-running` plus `receipts cluster-show` for live cluster-backed readback.

## Architecture

```
Applications     Forge, CI/CD, Secrets, DNS, Automerge, FUSE
Coordination     Locks, Elections, Queues, Barriers, Semaphores, Counters
Core             KV Store (Raft) + Blob Store (iroh-blobs) + Docs (iroh-docs)
Consensus        OpenRaft (vendored) + Redb (single-fsync writes)
Transport        Iroh QUIC (ALPN multiplexing, gossip, mDNS, DHT)
```

A single QUIC endpoint handles Raft replication, client RPCs, blob transfer, gossip, and federation through ALPN routing. No HTTP, no REST -- all communication goes through iroh.

Storage uses redb with the Raft log and state machine applied in a single transaction (one fsync per write batch). Concurrent client writes get batched into one Raft proposal. OpenRaft is vendored at `openraft/openraft` for direct patching.

Core traits are `ClusterController` (membership) and `KeyValueStore` (KV ops), both implemented by `RaftNode`.

## Coordination

Distributed primitives on the KV layer's compare-and-swap. All linearizable through Raft.

- `DistributedLock` / `DistributedRwLock` -- mutual exclusion with fencing tokens, reader-writer with fairness
- `LeaderElection` -- lease-based with automatic renewal
- `DistributedBarrier` -- N-party synchronization
- `Semaphore`, `AtomicCounter`, `SequenceGenerator` -- bounded access, counters, monotonic IDs
- `DistributedRateLimiter` -- token bucket
- `QueueManager` -- FIFO with visibility timeout, ack/nack, dead letter queue
- `ServiceRegistry` -- discovery with health checks
- `WorkerCoordinator` -- work stealing, load balancing, failover

```rust
let lock = DistributedLock::new(store, "my_lock", "client_1", LockConfig::default());
let guard = lock.acquire().await?;
let token = guard.fencing_token(); // pass to external services
// released on drop

let election = LeaderElection::new(store, "service-leader", "node-1", ElectionConfig::default());
let handle = election.start().await?;
if handle.is_leader() { /* lead */ }
```

Pure business logic in `src/verified/`, with formal proofs in `verus/` covering properties like fencing token monotonicity and mutual exclusion.

## Forge

Git hosting on Aspen's storage layers. Git objects go into iroh-blobs (BLAKE3), refs go into Raft KV. Issues, patches, and reviews stored as immutable DAGs. iroh-gossip announces new commits to peers.

```bash
git remote add aspen aspen://cluster-ticket/my-repo
git push aspen main
```

## CI/CD

Pipelines auto-trigger on Forge pushes. Three executor backends:

- **Shell** -- host-level, fast builds in pre-isolated environments
- **Nix** -- sandbox, reproducible flake builds, artifacts to iroh-blobs + binary cache
- **VM** -- Cloud Hypervisor microVM for untrusted workloads

Pipelines defined in [Nickel](https://nickel-lang.org/) (`.aspen/ci.ncl`). Jobs distributed across the cluster via the Raft-backed job queue.

## Federation

Independent clusters can sync over P2P. Each cluster runs its own consensus and works offline. Discovery uses BitTorrent Mainline DHT (BEP-44). Clusters identify with Ed25519 keypairs. Within a cluster: strong consistency via Raft. Across clusters: pull-based sync with cryptographic verification, eventual consistency.

Sync is application-level. Two Forge instances sync repos. Two CI systems share artifacts. The core provides transport and blob transfer; applications decide what to sync and when.

See [Federation Guide](docs/FEDERATION.md).

## AspenFS

FUSE filesystem that mounts a cluster as a POSIX directory. Paths map to KV keys.

```bash
aspen-fuse --mount-point /mnt/aspen --ticket <cluster-ticket>

echo "hello" > /mnt/aspen/myapp/config    # KV write
cat /mnt/aspen/myapp/config                # KV read
ls /mnt/aspen/myapp/                       # virtual directory from key prefixes
```

Also ships a VirtioFS backend for Cloud Hypervisor and QEMU. The VM CI executor uses this to give build jobs direct access to cluster storage.

## Usage

```bash
nix develop                    # dev shell
cargo build                    # build
cargo build --features full    # everything
```

```bash
# run a node
cargo run --features node-runtime-apps,blob,automerge \
  --bin aspen-node -- --node-id 1 --cookie my-cluster

# CLI
cargo run -p aspen-cli -- kv get mykey

# 3-node local cluster
nix run .#cluster

# self-hosted build pipeline
nix run .#dogfood-local -- full

# leave the verified dogfood cluster running for live Aspen KV receipt readback
nix run .#dogfood-local -- full --leave-running

# inspect durable self-hosting evidence from the latest local run
nix run .#dogfood-local -- --cluster-dir /tmp/aspen-dogfood receipts list
nix run .#dogfood-local -- --cluster-dir /tmp/aspen-dogfood receipts show <run-id>

# inspect live cluster-backed receipt evidence while the cluster is still running
nix run .#dogfood-local -- --cluster-dir /tmp/aspen-dogfood receipts cluster-show <run-id> --json
```

## Testing

```bash
cargo nextest run                                    # all tests
cargo nextest run -P quick                           # skip slow tests
cargo nextest run -E 'test(/raft/)'                  # filter
nix build .#checks.x86_64-linux.kv-operations-test   # NixOS VM test
nix run .#verify-verus                               # Verus proofs
```

madsim for deterministic simulation, proptest/Bolero for property-based testing and fuzzing, NixOS+QEMU for full cluster VM tests, buggify for fault injection, [Verus](https://github.com/verus-lang/verus) for formal verification (`verus/` specs, `src/verified/` code).

## Docs

- [Architecture](docs/developer-guide/architecture.md)
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

## References

- `../onix-site/` — UI design reference for Forge web. Use `design-system/colors_and_type.css` for theme/type tokens and `ui_kits/site/` for promoted Onix Computer layout, header/footer, card, chrome, and code-surface patterns.
