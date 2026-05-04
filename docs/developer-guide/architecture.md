# Aspen Architecture

This page is the top-level map of Aspen internals. Use it to orient yourself in the workspace, then dive into subsystem-specific docs for implementation details.

## System Overview

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│                                Entry Points                                  │
│                                                                              │
│  aspen-node        aspen-cli        aspen-tui        git-remote-aspen         │
│  aspen-dogfood     Forge Web        FUSE mount       CI agents/workers        │
│  snix bridge       Nix cache gateway                                         │
└──────────┬──────────────┬───────────────┬──────────────────────┬────────────┘
           │              │               │                      │
           ▼              ▼               ▼                      ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                         Iroh Endpoint + ALPN Router                          │
│                                                                              │
│  CLIENT_ALPN      RAFT_AUTH_ALPN  GOSSIP_ALPN     logs/net      blobs/docs   │
│  Client/TUI RPC   node RPC        discovery       streams       transfer/sync │
└──────────┬──────────────┬───────────────┬──────────────────────┬────────────┘
           │              │               │                      │
           ▼              ▼               ▼                      ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                         Handler + Application Layer                          │
│                                                                              │
│  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐ ┌────────────────────┐  │
│  │ Core RPC     │ │ Cluster RPC  │ │ App handlers │ │ Feature services   │  │
│  │ KV, watch,   │ │ membership,  │ │ Forge, CI,   │ │ hooks, proxy,      │  │
│  │ leases       │ │ metrics      │ │ jobs, docs   │ │ federation, net    │  │
│  └──────┬───────┘ └──────┬───────┘ └──────┬───────┘ └──────────┬─────────┘  │
└─────────┴────────────────┴────────────────┴────────────────────┴────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                       Distributed Primitive Layer                            │
│                                                                              │
│  Raft-backed KV     Coordination     Blob store     Docs/CRDT     Jobs       │
│  transactions       locks/queues     iroh-blobs     iroh-docs     queues     │
└──────────┬──────────────┬───────────────┬──────────────────────┬────────────┘
           │              │               │                      │
           ▼              ▼               ▼                      ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                         Consensus + Storage Layer                            │
│                                                                              │
│  openraft Raft group      redb unified log + state machine      snapshots     │
│  membership changes       one write transaction per batch       transfer      │
└─────────────────────────────────────────────────────────────────────────────┘
```

Aspen's invariant: cluster-wide mutable state goes through Raft, all network communication goes through Iroh, and durable local state goes through bounded storage paths. Higher-level systems are thin applications over those primitives.

## Directory Structure

```text
aspen/
├── Cargo.toml                    # workspace members, features, binary targets
├── src/                          # top-level aspen-node binary and compatibility shell
│   ├── bin/aspen_node/           # node startup, runtime wiring, CLI args
│   └── node/                     # node service setup helpers
│
├── crates/
│   ├── aspen-constants/          # Tiger Style limits shared across crates
│   ├── aspen-kv-types/           # alloc-safe KV key/value types
│   ├── aspen-storage-types/      # portable storage DTOs
│   ├── aspen-hlc/                # hybrid logical clock types
│   ├── aspen-time/               # wall-clock boundary
│   ├── aspen-traits/             # core trait contracts
│   ├── aspen-core/               # alloc-focused core types and verified helpers
│   ├── aspen-core-shell/         # std/runtime helpers exported as aspen-core for shells
│   ├── aspen-auth-core/          # alloc-safe capabilities and token types
│   ├── aspen-ticket/             # portable ticket model
│   ├── aspen-hooks-types/        # hook config/event schema
│   ├── aspen-hooks-ticket/       # hook trigger ticket type
│   │
│   ├── aspen-client-api/         # postcard client request/response enums
│   ├── aspen-forge-protocol/     # Forge protocol/wire DTOs
│   ├── aspen-jobs-protocol/      # alloc/no-std jobs protocol DTOs
│   ├── aspen-coordination-protocol/ # coordination protocol DTOs
│   ├── aspen-dht-discovery/      # DHT discovery DTOs/helpers
│   ├── aspen-dag/                # DAG helper types
│   │
│   ├── aspen-raft-types/         # shared OpenRaft app config and Raft types
│   ├── aspen-raft-kv-types/      # Raft KV command/result wire types
│   ├── aspen-raft-kv/            # KV state machine logic
│   ├── aspen-redb-storage/       # redb tables, log, state machine persistence
│   ├── aspen-raft-network/       # node-to-node Raft transport over Iroh
│   ├── aspen-raft/               # RaftNode, storage adapter, Raft network plumbing
│   │
│   ├── aspen-transport/          # ALPN constants and transport helpers
│   ├── aspen-cluster-types/      # cluster node/address types
│   ├── aspen-cluster/            # endpoint manager, bootstrap, gossip, discovery
│   ├── aspen-rpc-core/           # protocol context and handler registry core
│   ├── aspen-rpc-handlers/       # central client RPC dispatch
│   ├── aspen-client/             # typed client transport
│   ├── aspen-core-essentials-handler/ # core client RPC handlers
│   ├── aspen-cluster-handler/    # cluster client RPC handlers
│   │
│   ├── aspen-coordination/       # locks, queues, barriers, semaphores, elections
│   ├── aspen-blob/               # iroh-blobs-backed content store
│   ├── aspen-blob-handler/       # blob client RPC handlers
│   ├── aspen-docs/               # iroh-docs CRDT integration
│   ├── aspen-docs-handler/       # docs client RPC handlers
│   ├── aspen-forge/              # Git hosting, refs, COBs, gossip, sync
│   ├── aspen-forge-handler/      # Forge client RPC handlers
│   ├── aspen-commit-dag/         # commit DAG service
│   ├── aspen-kv-branch/          # branch state over KV
│   │
│   ├── aspen-jobs-core/          # portable jobs model, payload, wire, keyspace helpers
│   ├── aspen-jobs/               # Raft-backed job queue and worker coordination
│   ├── aspen-jobs-guest/         # guest-side job helper crate
│   ├── aspen-jobs-worker-*/      # worker adapters for blob/maintenance/replication/shell/sql
│   ├── aspen-job-handler/        # jobs client RPC handlers
│   ├── aspen-ci-core/            # portable CI config/log/route helpers
│   ├── aspen-ci/                 # orchestrator, trigger service, config loading
│   ├── aspen-ci-handler/         # CI client RPC handlers
│   ├── aspen-ci-executor-*/      # shell, VM, and Nix CI executors
│   │
│   ├── aspen-snix/               # snix BlobService/DirectoryService/PathInfoService
│   ├── aspen-snix-bridge/        # nix-daemon bridge
│   ├── aspen-nix-cache-gateway/  # Nix binary cache gateway
│   ├── aspen-castore/            # content-addressed store helpers
│   ├── aspen-cache/              # cache service helpers
│   ├── aspen-exec-cache/         # execution cache helpers
│   │
│   ├── aspen-disk/               # disk/runtime storage helpers
│   ├── aspen-layer/              # layering abstractions
│   ├── aspen-sharding/           # sharding helpers
│   ├── aspen-proxy/              # reverse proxy support
│   ├── aspen-net/                # network utility crate
│   ├── aspen-federation/         # cross-cluster sync abstractions
│   ├── aspen-trust/              # Shamir trust quorum and epoch state
│   ├── aspen-secrets/            # encrypted secrets at rest and rotation support
│   ├── aspen-secrets-handler/    # secrets client RPC handlers
│   ├── aspen-auth/               # runtime token verifier/builder shell
│   │
│   ├── aspen-cli/                # command-line client
│   ├── aspen-tui/                # terminal UI
│   ├── aspen-dogfood/            # self-hosted Forge + CI + Nix pipeline driver
│   ├── aspen-forge-web/          # web UI for Forge and CI views
│   ├── aspen-fuse/               # POSIX mount over KV namespace
│   ├── aspen-testing-core/       # reusable test helpers
│   ├── aspen-testing/            # test support facade
│   └── aspen-testing-*/          # fixtures, madsim, network, patchbay test support
│
├── openraft/                     # vendored openraft and macros
├── nix/                          # flake modules and NixOS VM integration tests
├── docs/                         # subsystem docs, ADRs, reference material
├── openspec/                     # active and archived structured changes
├── scripts/                      # repo guardrails and verification helpers
└── vendor/                       # patched upstream dependencies
```

The workspace is intentionally split by boundary: alloc-safe value crates at the bottom, runtime shells at the edge, and feature-gated applications above the consensus/transport core.

## Data Flow

### Client KV Write

```text
aspen-cli kv put
  → aspen-client opens Iroh stream with CLIENT_ALPN
  → ClientProtocolHandler decodes ClientRpcRequest::WriteKey
  → handler validates capability/rate limits/request metadata
  → RaftNode.client_write() proposes KV command through openraft
  → leader replicates log entry to quorum over RAFT_AUTH_ALPN or legacy RAFT_ALPN
  → redb transaction appends log and applies state-machine update
  → ClientRpcResponse::WriteResult returns over same Iroh stream
```

Reads that require linearizability go through Raft read paths. Prefix scans enforce fixed result and key/value bounds from the constants crates.

### Node Bootstrap

```text
aspen-node main
  → parse config and feature-gated service settings
  → build Iroh endpoint and ALPN router
  → open redb storage and create RaftNode
  → register client, Raft, gossip, TUI, blob/doc protocol handlers
  → start cluster discovery, metrics, worker services, and app runtimes
  → serve until shutdown signal or fatal bootstrap error
```

Bootstrap keeps transport concerns separate from application state. Node addresses and Iroh endpoint information are routing data; membership changes remain Raft state.

### Forge Push to CI Run

```text
git push via git-remote-aspen
  → Forge handler receives Git pack over Iroh client RPC
  → Git objects go to iroh-blobs by BLAKE3 hash
  → refs update through Raft KV for consensus
  → Forge gossip announces RefUpdate
  → CI TriggerService observes watched repo/ref
  → PipelineOrchestrator loads .aspen/ci.ncl
  → jobs enqueue in Aspen Jobs
  → shell/Nix/VM executor runs work and uploads logs/artifacts
  → status and attestations are written back to Forge/CI state
```

Forge owns Git semantics. CI owns pipeline semantics. Both use shared Aspen primitives instead of a separate database or external queue.

### Nix Binary Cache and Native Build

```text
nix build or CI Nix executor
  → aspen-ci-executor-nix evaluates/builds via snix when possible
  → outputs ingest into aspen-snix PathInfo/Directory/Blob services
  → nar-bridge or nix-daemon compatibility edge serves Nix clients
  → actual content reads use iroh-blobs and Raft-backed path metadata
```

The HTTP cache gateway and nix-daemon socket are compatibility edges for Nix clients. They are not Aspen's internal API surface.

### Federation Sync

```text
cluster-local app event
  → app publishes announcement over gossip/DHT-capable discovery
  → remote cluster verifies identity and capability metadata
  → remote app pulls missing content-addressed objects over Iroh
  → remote app applies its own merge/sync policy to local Raft state
```

Federation is application-level. Each cluster remains sovereign and strongly consistent internally; cross-cluster state is eventually consistent and cryptographically verified.

## Recommended Reading Order

If new to Aspen internals:

1. **This page** — orient yourself around layers and data flow.
2. **[README](../../README.md)** — project purpose, quick commands, feature overview.
3. **[ADRs](../adr/README.md)** — why Aspen is Iroh-only, vendored OpenRaft, redb-backed, and Tiger Style.
4. **[Tiger Style](../tigerstyle.md)** — coding constraints, resource bounds, assertion style.
5. **[No-std Core](../no-std-core.md)** — alloc/runtime boundary and portable core goals.
6. **[Forge](../forge.md)** — decentralized Git hosting over Aspen primitives.
7. **[Nix Integration](../nix-integration.md)** — snix store/eval/build/cache architecture.
8. **[Federation](../FEDERATION.md)** — cross-cluster identity, discovery, and sync.
9. **[Trust Quorum](../trust-quorum.md)** — cluster secret sharing and epoch management.
10. **[Observability](../observability.md)** — metrics exposed by handlers and runtime services.
11. **[Plugin Development](../PLUGIN_DEVELOPMENT.md)** — extension boundaries and host ABI.
12. **OpenSpec active changes** — current design intent under `openspec/changes/`.

## Major Subsystems

### Consensus Core

`RaftNode` is the production implementation of `ClusterController` and `KeyValueStore`. It wraps vendored OpenRaft, redb storage, snapshot transfer, follower forwarding, write batching, and membership changes.

State that must be linearizable enters as a Raft command. Storage commits the log and state-machine mutation in one redb transaction so a crash cannot persist one without the other.

→ [`crates/aspen-raft/`](../../crates/aspen-raft), [`crates/aspen-redb-storage/`](../../crates/aspen-redb-storage), [ADR 003](../adr/003-redb-unified-storage.md)

### Transport and Routing

Aspen uses one Iroh endpoint per node and routes protocols by ALPN. Client RPC, TUI RPC, Raft replication, gossip, log streams, net tunnels, blob transfer, and docs sync all share the transport boundary but keep separate protocol handlers.

There is no REST control plane. Any HTTP-facing crate exists as a compatibility adapter for another ecosystem, such as Nix binary cache clients or browser-facing Forge Web.

→ [`crates/aspen-transport/`](../../crates/aspen-transport), [`crates/aspen-cluster/`](../../crates/aspen-cluster), [ADR 009](../adr/009-alpn-protocol-routing.md)

### RPC and Client API

`aspen-client-api` defines append-only postcard request/response enums. `aspen-rpc-core` owns handler registry types and shared context. `aspen-rpc-handlers` routes client operations to feature-gated handler crates.

Compatibility rule: existing enum discriminants are stable. New non-gated variants are appended, not inserted into domain sections.

Routing metadata rule: every app-owned `ClientRpcRequest` variant must have an explicit required-app route in `crates/aspen-client-api/src/messages/request_metadata_apps/`. The source of truth is the API routing namespace prefix registry `APP_REQUEST_NAMESPACE_PREFIX_CONTRACTS` in `crates/aspen-client-api/src/messages/request_metadata_apps.rs`; `test_app_request_routing_tables_match_prefix_contracts` enforces that namespace-prefix ownership matches `required_app` routing. Native handler factories that serve an app-owned request must advertise the same app through `HandlerFactory::app_id()` so `HandlerRegistry::new` populates the cluster `AppRegistry` consistently with fallback/proxy routing; `native_handler_factories_advertise_their_required_app_namespace` keeps factory app IDs aligned with request routing metadata. App-serving native handlers must also accept their app-owned request variants through `RequestHandler::can_handle()` before fallback/proxy handling; `native_contacts_requests_reach_net_dispatch_path` and `native_deploy_requests_reach_cluster_dispatch_path` guard representative contacts/deploy dispatch paths. Authorization metadata rule: every `ClientRpcRequest` variant must also be explicitly classified in `crates/aspen-client-api/src/messages/to_operation/*.rs` as either requiring an `aspen_auth_core::Operation` or intentionally public (`Some(None)`). The drift guard `every_client_request_variant_has_authorization_classification` parses the request enum and fails if a new variant is not named in a to-operation classifier; this prevents new RPCs from silently becoming unauthenticated when `ClientProtocolContext.require_auth` is enabled. Runtime enforcement must consult this classification before handler dispatch: `handle_client_request_inner` calls `handle_client_request_check_auth` before `handle_client_request_dispatch`, and `client_request_auth_operation` fails closed for classified operations when `require_auth=true` but no token verifier is configured. Key-audience tokens are presenter-bound to the Iroh connection identity: `handle_client_connection_inner` passes `Connection::remote_id()` into `handle_client_request`, and `handle_client_request_check_auth` passes `Some(client_id)` into `TokenVerifier::authorize`; `client_request_path_uses_connection_remote_id_as_auth_presenter` guards that a token issued to one Iroh public key cannot be replayed by a different peer. When adding an app RPC, update the request metadata, the matching app routing table, the authorization classifier, and the native handler dispatch predicate in the same change; otherwise the request can compile at the API layer but route as a core/internal request, bypass auth classification, or fall through to capability-unavailable. Root-token issuance rule: `aspen_auth::generate_root_token`, `aspen-token generate-root`, and `aspen-node --output-root-token` intentionally create full-access bearer tokens for first-operator bootstrap. They are not bound to a client Iroh public key. Operators should use the root token to delegate narrower key-bound child tokens (`TokenBuilder::for_key` / `aspen-token delegate --audience`) for day-to-day clients, and docs/CLI output must state the audience explicitly.

Current app namespace prefixes are:

| App | Request variant prefixes |
|-----|--------------------------|
| `automerge` | `Automerge*` |
| `calendar` | `Calendar*` |
| `ci` | `Ci*` |
| `contacts` | `Contacts*`, `Net*` |
| `deploy` | `ClusterDeploy*`, `ClusterRollback*`, `NodeRollback*`, `NodeUpgrade*` |
| `forge` | `FederateRepository*`, `Federation*`, `Forge*`, `GetDiscoveredCluster*`, `GetFederationStatus*`, `GitBridge*`, `Gossip*`, `ListDiscoveredClusters*`, `ListFederatedRepositories*`, `StartGossip*`, `StopGossip*`, `TrustCluster*`, `UntrustCluster*` |
| `hooks` | `Hook*` |
| `jobs` | `Job*`, `Worker*` |
| `secrets` | `Secrets*` |
| `snix` | `Cache*`, `NixCache*`, `Snix*` |
| `sql` | `ExecuteSql*` |

→ [`crates/aspen-client-api/`](../../crates/aspen-client-api), [`crates/aspen-rpc-core/`](../../crates/aspen-rpc-core), [`crates/aspen-rpc-handlers/`](../../crates/aspen-rpc-handlers)

### Coordination Primitives

`aspen-coordination` builds locks, read-write locks, queues, barriers, semaphores, counters, rate limiters, service registry, and worker coordination on top of `KeyValueStore` compare-and-swap behavior.

The business rules live in pure `verified` modules. Async shells handle clocks, retries, store calls, and cancellation.

→ [`crates/aspen-coordination/`](../../crates/aspen-coordination), [ADR 004](../adr/004-functional-core-imperative-shell.md), [ADR 005](../adr/005-verus-two-file-architecture.md)

### Blob and Document Storage

`aspen-blob` integrates iroh-blobs for immutable content-addressed objects. `aspen-docs` integrates iroh-docs for CRDT document sync. Mutable pointers to content live in Raft KV; large immutable payloads move through content-addressed protocols.

This split keeps consensus entries small while preserving cryptographic integrity and deduplication.

→ [`crates/aspen-blob/`](../../crates/aspen-blob), [`crates/aspen-docs/`](../../crates/aspen-docs), [ADR 008](../adr/008-iroh-blobs-content-addressed-storage.md)

### Forge

Forge provides Git hosting, refs, collaborative objects, patches, reviews, issues, discussions, repo identity, gossip announcements, and federation sync. Git objects and COB changes are immutable blobs; refs and metadata are consensus-backed KV records.

→ [`crates/aspen-forge/`](../../crates/aspen-forge), [`crates/aspen-forge-handler/`](../../crates/aspen-forge-handler), [Forge docs](../forge.md)

### CI, Jobs, and Execution

Aspen CI loads Nickel pipeline config, triggers runs from Forge refs, schedules jobs through Aspen Jobs, and executes work through shell, Nix, or VM backends. Executors upload logs and artifacts back into Aspen storage.

Jobs are a general distributed work substrate. CI is one application on top.

→ [`crates/aspen-ci-core/`](../../crates/aspen-ci-core), [`crates/aspen-ci/`](../../crates/aspen-ci), [`crates/aspen-jobs-core/`](../../crates/aspen-jobs-core), [`crates/aspen-jobs-protocol/`](../../crates/aspen-jobs-protocol), [`crates/aspen-jobs/`](../../crates/aspen-jobs), [`crates/aspen-ci-handler/`](../../crates/aspen-ci-handler), [`crates/aspen-job-handler/`](../../crates/aspen-job-handler), [`crates/aspen-ci-executor-shell/`](../../crates/aspen-ci-executor-shell), [`crates/aspen-ci-executor-vm/`](../../crates/aspen-ci-executor-vm), [`crates/aspen-ci-executor-nix/`](../../crates/aspen-ci-executor-nix), [ADR 007](../adr/007-nickel-ci-configuration.md)

### Nix and snix

`aspen-snix` implements snix store traits over Aspen storage. `aspen-ci-executor-nix` evaluates and builds derivations in-process when possible. Compatibility binaries expose nar-bridge HTTP and nix-daemon protocols for existing Nix clients.

→ [`crates/aspen-snix/`](../../crates/aspen-snix), [`crates/aspen-snix-bridge/`](../../crates/aspen-snix-bridge), [`crates/aspen-nix-cache-gateway/`](../../crates/aspen-nix-cache-gateway), [Nix Integration](../nix-integration.md)

### Trust, Auth, and Secrets

Auth separates portable capability/token types from runtime verification. Trust quorum uses Shamir secret sharing to protect cluster secrets and manages epoch rotation. Secrets-at-rest preserves old epoch keys until background re-encryption finishes so mixed-epoch reads keep working.

→ [`crates/aspen-auth-core/`](../../crates/aspen-auth-core), [`crates/aspen-auth/`](../../crates/aspen-auth), [`crates/aspen-trust/`](../../crates/aspen-trust), [`crates/aspen-secrets/`](../../crates/aspen-secrets), [Trust Quorum](../trust-quorum.md)

### Federation

Federation connects independent clusters without merging their consensus groups. Discovery, identity, and blob transfer are shared; conflict resolution and sync policy stay in each application.

→ [`crates/aspen-federation/`](../../crates/aspen-federation), [`crates/aspen-cluster/src/federation/`](../../crates/aspen-cluster/src/federation), [Federation Guide](../FEDERATION.md)

### Interfaces and Operators

`aspen-cli` is the typed operator/client surface. `aspen-tui` provides live terminal visibility. `aspen-forge-web` is a browser UI for Forge/CI. `aspen-fuse` maps a KV namespace into a POSIX filesystem. `aspen-dogfood` proves self-hosting by driving cluster, Forge, CI, Nix build, deploy, and verification flows.

→ [`crates/aspen-cli/`](../../crates/aspen-cli), [`crates/aspen-tui/`](../../crates/aspen-tui), [`crates/aspen-forge-web/`](../../crates/aspen-forge-web), [`crates/aspen-fuse/`](../../crates/aspen-fuse), [`crates/aspen-dogfood/`](../../crates/aspen-dogfood)

### Verification and Testing

Aspen combines unit/integration tests, deterministic madsim simulations, NixOS VM tests, property tests, fuzzing, Verus specs, and OpenSpec evidence. Pure core logic belongs in `src/verified/`; formal proofs live beside crates under `verus/` where applicable.

→ [Tiger Style](../tigerstyle.md), [ADR 010](../adr/010-madsim-deterministic-simulation.md), [`nix/tests/`](../../nix/tests), [`openspec/`](../../openspec)

## Design Principles

| Principle | What it means in practice |
|-----------|---------------------------|
| **Iroh-only internal networking** | Client RPC, Raft, gossip, TUI, blob transfer, docs sync, and federation route over Iroh/QUIC with ALPN. HTTP is only a compatibility edge, not the internal control plane. |
| **Raft for cluster-wide mutable state** | Membership, refs, KV data, jobs, secrets metadata, and other authoritative cluster state go through consensus. Local caches and blobs are not authority by themselves. |
| **Content-addressed large data** | Git objects, build artifacts, NAR payloads, COB changes, and other immutable content use BLAKE3-addressed blob paths while mutable heads live in Raft. |
| **Functional core, imperative shell** | Pure deterministic logic lives in `verified` modules. Async runtimes, I/O, clocks, storage, and networking stay in thin shell code. |
| **Bounded operations** | Scans, batches, key sizes, value sizes, peers, queues, timeouts, and retries use named constants and fixed limits. No unbounded distributed operation enters production paths. |
| **Feature-gated surfaces** | Optional subsystems compile in only when their feature is enabled. Public types that mention optional crates model that dependency explicitly. |
| **Alloc-first lower layers** | Foundational types avoid `std` where possible. Runtime shells add filesystems, sockets, time, and background tasks at the edge. |
| **Evidence-backed changes** | Non-trivial changes use tests, OpenSpec task evidence, saved transcripts, and review gates instead of chat-only claims. |
| **Dogfood pressure** | Forge, CI, Nix cache, deployment, and verification are designed to build Aspen using Aspen itself. |

## File Dependency Chains

### Core Runtime Chain

```text
aspen-kv-types / aspen-storage-types / aspen-cluster-types
       ↑
aspen-core + aspen-core-shell
       ↑
aspen-raft-types → aspen-raft-kv-types → aspen-raft-kv
       ↑
aspen-redb-storage + aspen-raft-network
       ↑
aspen-raft::RaftNode
       ↑
aspen-cluster bootstrap + Iroh router
       ↑
aspen-rpc-core + aspen-rpc-handlers
       ↑
aspen-node / aspen-cli / aspen-tui / application handlers
```

Lower crates should not depend on runtime application crates. If a pure type starts pulling in I/O, split a shell crate rather than widening the foundation.

### Client RPC Chain

```text
aspen-client-api messages
       ↑
aspen-client transport codec
       ↑
Iroh CLIENT_ALPN stream
       ↑
HandlerRegistry dispatch
       ↑
feature handler crate
       ↑
RaftNode / blob store / docs store / app service
```

Request/response enum order is a compatibility contract. Handler routing can be feature-gated; wire discriminants cannot move.

### Application Chain

```text
shared primitives: KeyValueStore + ClusterController + BlobStore + Docs
       ↑
coordination / jobs / auth / secrets / forge / ci / federation / snix
       ↑
handler crates expose RPC operations
       ↑
aspen-node wires enabled services into ClientProtocolContext
       ↑
operators use aspen-cli, git-remote-aspen, Forge Web, FUSE, dogfood
```

Applications own their domain state and sync semantics, but they should not create a second consensus, queue, or transport substrate.

## Rules of Thumb for New Work

- Put deterministic decisions in a pure helper first; call it from async shell code.
- Pass time, randomness, node identity, and limits as explicit inputs to pure logic.
- Route network traffic through Iroh and add an ALPN or client RPC variant when needed.
- When adding an app-owned `ClientRpcRequest`, preserve the app prefix contract and update `request_metadata_apps/<app>.rs` so `required_app` routing cannot fall through to core handling.
- Store authoritative mutable state through Raft; store large immutable bytes in blobs.
- Keep public feature gates honest: if a public type names an optional crate, the feature must pull it in.
- Add positive and negative tests for new behavior, including malformed input and bound violations.
- Update subsystem docs when changing cross-crate boundaries or data flow.
