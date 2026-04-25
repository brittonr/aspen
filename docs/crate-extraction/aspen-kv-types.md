# Extraction Manifest: aspen-kv-types

## Candidate

- **Family**: Redb Raft KV
- **Canonical class**: `leaf type/helper`
- **Canonical crate/path**: `crates/aspen-kv-types`
- **Intended audience**: Rust projects that need Aspen-compatible KV command, response, transaction, lease, and validation types without Aspen node runtime.
- **Public API owner**: Aspen KV types maintainers
- **Readiness state**: `workspace-internal`
- **Dependency policy class**: reusable library candidate

## Package and release metadata

- **Package description**: Reusable KV operation, response, transaction, lease, and validation types for Aspen-style KV APIs.
- **Documentation entrypoint**: crate-level Rustdoc plus examples in downstream consumer fixture.
- **License policy**: AGPL-3.0-or-later until human license strategy changes.
- **Repository/homepage policy**: monorepo path until publication policy is decided.
- **Semver/compatibility policy**: no external semver guarantee yet; maintain in-workspace compatibility until `extraction-ready-in-workspace`.
- **Publish readiness**: blocked; do not mark publishable during this change.

## Feature contract

| Feature set | Status | Purpose |
| --- | --- | --- |
| default | reusable default | KV types and validation helpers only. |
| testing/dev | dev-only | Property, snapshot, and wire-compat tests. |

Default features must not require root `aspen`, binaries, handler registries, dogfood, UI, trust, secrets, SQL, coordination runtime, or concrete iroh endpoint construction.

## Dependencies

### Internal Aspen dependencies

| Dependency | Decision | Reason |
| --- | --- | --- |
| `aspen-constants` | keep | Shared bounded sizes and validation limits are part of the KV type contract. |

### External dependencies

Serialization and validation dependencies are allowed if they are required by the public wire/type contract and remain documented in Cargo metadata.

### Binary/runtime dependencies

None allowed in default reusable features.

## Compatibility and aliases

- **Old path**: current canonical path is already `aspen_kv_types::*`.
- **New path**: unchanged.
- **Compatibility re-exports**: none planned unless callers currently import KV types through `aspen_raft` or root `aspen` paths.
- **Removal criteria**: any future compatibility re-export is removable after in-repo consumers and downstream fixture use `aspen_kv_types` directly.

## Representative consumers and re-exporters

- `aspen-raft`
- `aspen-coordination`
- `aspen-jobs`
- `aspen-kv-branch`
- downstream Redb Raft KV consumer fixture

## Dependency exceptions

| candidate | feature_set | dependency_path | owner | reason |
| --- | --- | --- | --- | --- |
| `aspen-kv-types` | default | `aspen-kv-types -> aspen-constants` | Aspen KV types maintainers | KV validation limits are reusable contract data, not app runtime. |

## Verification rails

- `cargo check -p aspen-kv-types --no-default-features`
- `cargo check -p aspen-kv-types`
- serialization/wire compatibility tests for public request/response types
- dependency-boundary checker for direct, transitive, representative-consumer, and re-export leaks
- positive downstream example importing `aspen_kv_types` directly
- negative boundary check proving app-only APIs are unavailable from this crate

## First-slice status

Current status is `workspace-internal`. Before this layer can be marked ready, the feature matrix, downstream consumer metadata, and dependency-boundary evidence must be linked from `openspec/changes/prepare-crate-extraction/verification.md`.
