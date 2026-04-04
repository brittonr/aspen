## Why

The test suite has structural gaps that reduce confidence in correctness. `aspen-secrets-handler` (1,082 lines of security-critical code) has zero tests. Several large crates have fewer than 1 test per 300 lines. There's no mutation testing to verify existing tests actually catch regressions. Proptest coverage is thin (25 test blocks for 436k lines), and madsim scenarios miss important failure modes like split-brain heal and snapshot-during-membership-change.

## What Changes

- Add mutation testing via `cargo-mutants` to CI, starting with `aspen-coordination`, `aspen-core`, and `aspen-raft` verified modules
- Write unit and integration tests for `aspen-secrets-handler` and `aspen-snix-bridge` (currently at 0 tests)
- Expand proptest model-based tests for Raft storage, coordination primitives (queues, barriers, elections), and KV linearizability
- Add madsim deterministic simulation scenarios for split-brain heal, slow follower, snapshot-during-membership-change, and clock-skewed TTL checks
- Add fault injection points at redb write, Iroh connection, snapshot transfer, and blob download boundaries
- Add serialization snapshot tests (via `insta`) for RPC wire formats and CLI output to detect accidental format changes
- Increase test density in under-tested handler crates (`aspen-forge-handler`, `aspen-ci-handler`, `aspen-docs-handler`)

## Capabilities

### New Capabilities

- `mutation-testing`: Mutation testing infrastructure using `cargo-mutants`, CI integration, and baseline mutation scores for verified crates
- `fault-injection-coverage`: Systematic fault injection points at I/O boundaries (redb, Iroh, snapshots, blobs) with buggify integration
- `serialization-snapshots`: Insta snapshot tests for RPC message serialization, CLI output formatting, and error message chains

### Modified Capabilities

- `coordination`: Expanded proptest model-based coverage for queues, barriers, elections, and rate limiters
- `consensus`: New madsim scenarios for split-brain heal, slow follower, snapshot-during-membership-change, and clock-skewed TTL

## Impact

- **Crates with new tests**: `aspen-secrets-handler`, `aspen-snix-bridge`, `aspen-forge-handler`, `aspen-ci-handler`, `aspen-docs-handler`
- **Crates with expanded tests**: `aspen-coordination`, `aspen-raft`, `aspen-core`, `aspen-redb-storage`, `aspen-cluster`
- **Build/CI**: New `cargo-mutants` step, new nextest profile for mutation runs
- **Dependencies**: `cargo-mutants` (dev tool, not a crate dependency), `insta` (dev-dependency for snapshot tests)
- **Test runtime**: Mutation testing adds significant CI time (run on schedule or gate on verified crates only)
