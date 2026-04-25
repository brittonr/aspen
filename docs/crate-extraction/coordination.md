# Extraction Manifest: Coordination

## Candidate

- **Family**: Coordination
- **Canonical class**: `service library` (aspen-coordination), `protocol/wire` (aspen-coordination-protocol)
- **Canonical crate/path**: `crates/aspen-coordination`, `crates/aspen-coordination-protocol`
- **Intended audience**: Rust projects that want distributed coordination primitives (locks, elections, queues, barriers, semaphores, rate limiters, counters, service registry, worker coordination) built generically over any `KeyValueStore` backend.
- **Public API owner**: Aspen coordination maintainers
- **Readiness state**: `extraction-ready-in-workspace`
- **Dependency policy class**: reusable service library candidate

## Package and release metadata

- **Package description**: Distributed coordination primitives generic over `KeyValueStore`. Includes locks with fencing tokens, leader elections, priority queues, barriers, counting semaphores, token-bucket rate limiters, distributed counters, service registry, and worker coordination with work stealing.
- **Documentation entrypoint**: Crate-level Rustdoc plus downstream consumer fixture proving standalone usage.
- **License policy**: AGPL-3.0-or-later until human license strategy changes.
- **Repository/homepage policy**: Monorepo path until publication policy is decided.
- **Semver/compatibility policy**: No external semver guarantee yet; coordination primitive APIs become semver-relevant once ready.
- **Publish readiness**: Blocked; do not mark publishable during this change.

## Feature contract

| Feature set | Status | Purpose |
| --- | --- | --- |
| default | reusable default | All coordination primitives generic over `KeyValueStore`, with no concrete transport, storage backend, or Aspen app runtime. |
| verus | dev/verification | Enables Verus formal verification ghost code in `src/verified/` modules. |
| bolero | dev-only | Enables property-based testing with Bolero. |
| trust/secrets/sql/transport/handler-registry | forbidden by default | Aspen app concerns live in integration crates or explicit opt-in features. |

## Verus formal verification

The `verus/` directory (28 spec files) and `src/verified/` modules (20 modules) are integral to the crate. They verify correctness invariants for locks, elections, queues, barriers, semaphores, rate limiters, and counters. These have no external Aspen dependencies and stay in-crate.

## Dependencies

### Internal Aspen dependencies

| Dependency | Decision | Reason |
| --- | --- | --- |
| `aspen-kv-types` | keep | `ReadRequest`, `ScanRequest`, `KvEntry` types for KV operations. |
| `aspen-traits` | keep, leaf-only | `KeyValueStore` trait; the generic bound for all coordination primitives. |
| `aspen-constants` | keep | Resource limits and validation thresholds. |
| `aspen-time` | keep | Explicit time injection for deterministic TTL/deadline computation. |
| `aspen-core` | removed | Was only used for `ReadRequest`/`ScanRequest` re-exports from `aspen-kv-types`. |
| `aspen-core-shell`, `aspen-auth`, `aspen-client-api`, handlers | never depended | Not in dependency graph. |

### External dependencies

| Dependency | Decision | Reason |
| --- | --- | --- |
| `async-trait` | keep | Async trait definitions for coordination primitives. |
| `serde` / `serde_json` | keep | Serialization of coordination state (lock entries, queue items, election terms). |
| `tracing` | keep | Observability instrumentation. |
| `snafu` | keep | Structured error handling with context. |
| `tokio` | keep | Async runtime for coordination operations (timers, sleep, spawn). |
| `uuid` | keep | Unique identifiers for lock holders, queue items. |

### Binary/runtime dependencies

No binaries. No concrete transport or storage backend required.

## Protocol crate

`aspen-coordination-protocol` is a standalone wire crate with only `serde` as a dependency. It defines coordination request/response types for RPC serialization. It has no Aspen dependencies.

## Compatibility and aliases

- **Old paths**: No compatibility re-exports needed. All consumers import directly from `aspen_coordination::*`.
- **New path**: Same `aspen_coordination::*` paths.
- **Compatibility re-exports**: None required; public API is already self-contained.
- **Removal criteria**: N/A.

## Representative consumers and re-exporters

- `aspen-cluster` (direct consumer)
- `aspen-core-essentials-handler` (direct consumer)
- `aspen-job-handler` (direct consumer)
- `aspen-jobs` (direct consumer)
- `aspen-rpc-core` (direct consumer)
- `aspen-rpc-handlers` (direct consumer)
- `aspen-testing` (optional consumer)
- `aspen-raft` (optional, behind `coordination` feature)
- `aspen-client-api` (protocol crate consumer only)

## Dependency exceptions

| candidate | feature_set | dependency_path | owner | reason |
| --- | --- | --- | --- | --- |
| `aspen-coordination` | default | `aspen-coordination -> tokio` | Aspen coordination maintainers | Async runtime for timer-based coordination (lock TTL, election timeouts, rate limiter refill). |

## Verification rails

- `cargo check -p aspen-coordination --no-default-features`
- `cargo check -p aspen-coordination` for default reusable features
- `cargo tree -p aspen-coordination` must not include `aspen-core`, `aspen-core-shell`, root `aspen`, or any handler/binary crate
- `cargo tree -p aspen-coordination-protocol` must not include any `aspen-*` crate
- positive downstream consumer fixture compiling with only `aspen-coordination`, `aspen-kv-types`, and `aspen-traits`
- negative boundary check proving app-only APIs and forbidden crates stay absent from the default dependency graph
- dependency-boundary checker for direct/transitive app-bundle and runtime transport leaks
- All 9 workspace consumers compile after dependency cleanup
- `cargo nextest run -p aspen-coordination` passes all existing tests
- Extraction-readiness checker with `--candidate-family coordination` exits 0

## First-slice status

Current status is `extraction-ready-in-workspace`. The `aspen-coordination` default feature set exposes all coordination primitives generic over `KeyValueStore` without Aspen app bundles, concrete transport, or runtime configuration. All extraction-readiness criteria are met:

- `aspen-core` dependency removed; replaced with direct `aspen-kv-types` imports
- Default features compile without root `aspen` or app bundles
- No forbidden dependencies in default graph
- Protocol crate (`aspen-coordination-protocol`) is fully standalone with only `serde`
- 28 Verus spec files and 20 verified modules stay in-crate with no external dependencies

Publishable/repo-split labels remain blocked until license/publication policy is decided.
