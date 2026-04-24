# Extraction Manifest: aspen-raft-network

## Candidate

- **Family**: Redb Raft KV
- **Canonical class**: `runtime adapter`
- **Canonical crate/path**: `crates/aspen-raft-network`
- **Intended audience**: Rust projects that want the reusable Redb Raft KV stack over Aspen's iroh/IRPC transport adapter.
- **Public API owner**: owner needed
- **Readiness state**: `workspace-internal`
- **Dependency policy class**: explicit runtime adapter candidate

## Package and release metadata

- **Package description**: iroh/IRPC network adapter for OpenRaft-based Aspen KV consensus.
- **Documentation entrypoint**: crate-level Rustdoc plus adapter wiring example.
- **License policy**: AGPL-3.0-or-later until human license strategy changes.
- **Repository/homepage policy**: monorepo path until publication policy is decided.
- **Semver/compatibility policy**: no external semver guarantee yet; ALPN/RPC message and OpenRaft network types become compatibility-relevant once ready.
- **Publish readiness**: blocked; do not mark publishable during this change.

## Feature contract

| Feature set | Status | Purpose |
| --- | --- | --- |
| default | adapter default | iroh/IRPC adapter for reusable KV stack. |
| sharding | named adapter feature | Sharding-aware routing if retained. |
| testing/dev | dev-only | madsim/property fixtures. |

This crate is allowed to depend on concrete iroh because it is the explicit adapter. Storage and consensus facade crates must compile without this crate unless an adapter feature is selected.

## OpenRaft boundary

OpenRaft network traits/types are adapter API for this crate. The manifest must record public OpenRaft exposure, and the adapter must not drag Aspen node bootstrap or handler registry into storage/facade defaults.

## Dependencies

### Internal Aspen dependencies

| Dependency | Decision | Reason |
| --- | --- | --- |
| current `aspen-raft-types` / future `aspen-raft-kv-types` | keep | OpenRaft app/network type metadata. |
| `aspen-transport` | keep if adapter-owned | ALPN/iroh transport constants and helpers. |
| `aspen-sharding` | gate or remove | Sharding is an optional Aspen routing concern. |
| `aspen-core` / `aspen-time` | review | Keep only leaf/time helpers needed by adapter. |
| `aspen-auth`, `aspen-core-shell`, handlers, binaries | forbid by default | App shell concerns do not belong in this adapter. |

### External dependencies

| Dependency | Decision | Reason |
| --- | --- | --- |
| `iroh` | keep | Adapter purpose. |
| `irpc` | keep | RPC framing used by adapter. |
| `openraft` | keep | OpenRaft network trait integration. |
| `tokio` / `async-trait` | keep | Async transport implementation. |
| `postcard` / `serde` | keep | Message serialization. |

### Binary/runtime dependencies

No binaries. Runtime dependency is allowed because this crate is explicitly the transport adapter.

## Compatibility and aliases

- **Old paths**: reusable portions of `aspen_raft::network::*`.
- **New path**: `aspen_raft_network::*`.
- **Compatibility re-exports**: `aspen_raft` re-exports old network paths during migration or migrates all in-repo callers.
- **Owner**: owner needed.
- **Tests**: compile old and new paths; keep encoding/property tests.
- **Removal criteria**: in-repo consumers and downstream adapter example import `aspen_raft_network` directly.

## Representative consumers and re-exporters

- `aspen-raft-kv` adapter feature
- `aspen-raft` compatibility crate
- `aspen-cluster`
- downstream Redb Raft KV iroh adapter fixture

## Dependency exceptions

| candidate | feature_set | dependency_path | owner | reason |
| --- | --- | --- | --- | --- |
| `aspen-raft-network` | default | `aspen-raft-network -> iroh` | owner needed | Adapter purpose. |
| `aspen-raft-network` | default | `aspen-raft-network -> irpc` | owner needed | Adapter message framing. |
| `aspen-raft-network` | default | `aspen-raft-network -> openraft` | owner needed | OpenRaft network trait integration. |
| `aspen-raft-network` | sharding | `aspen-raft-network -> aspen-sharding` | owner needed | Optional sharding-aware route metadata. |

## Verification rails

- `cargo check -p aspen-raft-network --no-default-features`
- `cargo check -p aspen-raft-network`
- adapter property tests for message encoding/decoding
- dependency-boundary checker proving adapter ownership of iroh while storage/facade defaults do not depend on adapter
- positive downstream example using `aspen_raft_network` explicitly
- negative boundary check proving storage/facade crates do not construct iroh endpoints without this adapter
- Aspen compatibility compile for old `aspen_raft::network::*` paths if re-exported

## First-slice status

Current status is `workspace-internal`. I9 evidence keeps this crate as the explicit adapter boundary: storage, reusable app types, and facade defaults compile without `aspen-raft-network`, iroh, IRPC, or transport dependencies, while this crate remains the place where concrete iroh/IRPC adapter dependencies appear. The adapter still reaches `aspen-transport`, `aspen-sharding`, `aspen-core`, and transitive app/runtime concerns; those paths require explicit policy ownership or feature gates before this adapter can be marked ready.
