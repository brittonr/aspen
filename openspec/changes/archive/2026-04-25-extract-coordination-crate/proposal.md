## Why

`aspen-coordination` (~22k lines) implements distributed primitives (locks, elections, queues, barriers, semaphores, rate limiters, service registry) that are generic over `KeyValueStore` and have no structural dependency on Aspen's node runtime, transport, or storage backend. Its only Aspen dependency is on leaf type/trait crates (`aspen-core`, `aspen-constants`, `aspen-kv-types`, `aspen-traits`, `aspen-time`), and it already has 28 Verus formal verification spec files proving correctness invariants. This makes it the highest-value next extraction target after the Redb Raft KV stack: a genuinely reusable distributed coordination library that works with any KV backend.

The thin `aspen-core` usage (only `ReadRequest`/`ScanRequest` from `aspen-kv-types`) means the dependency on `aspen-core` can be replaced with direct `aspen-kv-types` imports, making the crate fully independent of Aspen's core shell layer.

## What Changes

- Replace `aspen-coordination`'s dependency on `aspen-core` with direct `aspen-kv-types` imports for `ReadRequest`/`ScanRequest`
- Add extraction manifest at `docs/crate-extraction/coordination.md` following the established contract
- Add `aspen-coordination` and `aspen-coordination-protocol` to the extraction-readiness checker policy
- Prove default features compile without Aspen app bundles, handlers, binaries, or runtime transport
- Prove downstream-style consumer can use coordination primitives with only reusable KV trait deps
- Verify all 9 reverse-dep consumers still compile through direct imports or compatibility re-exports
- Update `docs/crate-extraction.md` inventory with verified readiness state

## Capabilities

### New Capabilities
- `coordination-extraction`: Extraction boundary, dependency contract, and standalone verification rails for `aspen-coordination` and `aspen-coordination-protocol` as reusable crates independent of Aspen runtime.

### Modified Capabilities
- `architecture-modularity`: Updates extraction inventory with coordination family readiness state and manifest links.

## Impact

- **Crates modified**: `aspen-coordination` (dependency cleanup), `aspen-coordination-protocol` (verification only)
- **Docs modified**: `docs/crate-extraction.md`, `docs/crate-extraction/policy.ncl`, new `docs/crate-extraction/coordination.md`
- **Consumers verified**: `aspen-cluster`, `aspen-core-essentials-handler`, `aspen-job-handler`, `aspen-jobs`, `aspen-raft` (optional), `aspen-rpc-core`, `aspen-rpc-handlers`, `aspen-testing` (optional), `aspen-client-api` (protocol only)
- **No breaking changes**: All consumer imports continue to work; `aspen-core` removal from coordination's deps is internal
