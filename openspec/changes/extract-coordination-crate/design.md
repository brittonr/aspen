## Context

`aspen-coordination` provides distributed primitives (locks, elections, queues, barriers, semaphores, rate limiters, counters, service registry, worker coordination) built generically over the `KeyValueStore` trait. It has ~22k lines, 20 verified modules, 28 Verus spec files, and 9 workspace consumers.

The Redb Raft KV extraction (`prepare-crate-extraction`) established the extraction contract, readiness checker, policy, and manifest infrastructure. This change applies the same framework to coordination.

Current dependency chain:
```
aspen-coordination → aspen-core (only ReadRequest/ScanRequest usage)
                   → aspen-constants
                   → aspen-kv-types (ReadRequest, ScanRequest, KvEntry already here)
                   → aspen-traits (KeyValueStore)
                   → aspen-time
```

`aspen-coordination-protocol` is already a standalone wire crate with only `serde` as a dependency.

## Goals / Non-Goals

**Goals:**
- Remove `aspen-coordination`'s dependency on `aspen-core`, replacing with direct `aspen-kv-types` imports
- Create extraction manifest documenting dependency contract, feature surface, and verification rails
- Add coordination candidates to the extraction-readiness policy and checker
- Prove default features compile without Aspen app bundles or runtime
- Prove a downstream-style consumer can use coordination primitives without depending on the root `aspen` package
- Verify all 9 reverse-dep consumers still compile
- Achieve `extraction-ready-in-workspace` readiness state for both `aspen-coordination` and `aspen-coordination-protocol`

**Non-Goals:**
- Publishing to crates.io (blocked on license/publication policy, same as Redb Raft KV)
- Repository split (deferred until publication policy decided)
- Renaming the crate (stays `aspen-coordination` inside the workspace)
- Changing the coordination API surface or adding new primitives
- Extracting `aspen-coordination`'s consumers (jobs, cluster, handlers stay in-tree)

## Decisions

### Decision 1: Replace `aspen-core` with `aspen-kv-types` directly

`aspen-coordination` uses `aspen-core` only for `ReadRequest` and `ScanRequest`, which are re-exports from `aspen-kv-types`. Switch to importing from `aspen-kv-types` directly.

**Alternative**: Keep `aspen-core` dependency. Rejected because `aspen-core` pulls `aspen-core-shell` in representative workspace consumers, leaking runtime concerns into the coordination crate's reusable graph.

### Decision 2: No compatibility re-exports needed

Unlike the Redb Raft KV extraction, coordination's public API is already self-contained. Consumers import `aspen_coordination::DistributedLock`, `aspen_coordination::QueueManager`, etc. No paths from `aspen-core` or other crates re-export coordination types. No compatibility shim is needed.

**Alternative**: Add re-exports anyway for safety. Rejected because grep shows no external paths re-exporting coordination types.

### Decision 3: Add both crates to extraction policy as a `coordination` family

Register `aspen-coordination` as `service library` class and `aspen-coordination-protocol` as `protocol/wire` class in `policy.ncl`. The protocol crate is already clean (only `serde`). The service crate needs the `aspen-core` removal verified.

### Decision 4: Downstream consumer proof uses in-tree fixture

Follow the Redb Raft KV pattern: add a small in-tree example or test that depends only on `aspen-coordination`, `aspen-kv-types`, and `aspen-traits` without going through root `aspen` or any handler/binary crate.

**Alternative**: Out-of-tree consumer. Rejected because in-tree fixtures are easier to maintain and are already the established pattern from the Redb Raft KV extraction.

### Decision 5: Verus specs stay in-crate

The `verus/` directory and `src/verified/` modules stay inside `aspen-coordination`. They are integral to the crate's value proposition and have no external Aspen dependencies.

## Risks / Trade-offs

- **[Risk] `aspen-core` removal breaks hidden usage** → Mitigation: grep-verified that only `ReadRequest`/`ScanRequest` are used; both live in `aspen-kv-types`.
- **[Risk] Representative consumer feature unification pulls app deps** → Mitigation: Run extraction-readiness checker with `aspen-cluster` and `aspen-rpc-handlers` as representative consumers.
- **[Risk] `aspen-traits` transitive leak** → Mitigation: `aspen-traits` is already verified as alloc-safe in the no-std boundary work; re-verify with `cargo tree` for coordination's specific feature set.
- **[Trade-off] Keeping `aspen-` prefix for unpublished crate** → Acceptable; rename is a future concern for publication policy. The prefix communicates provenance while in the monorepo.
