# Extraction Manifest: Blob / Castore / Cache

## Candidate

- **Family**: Blob/castore/cache
- **Canonical class**: `service library` (`aspen-blob`, `aspen-cache`) plus `runtime adapter` (`aspen-castore` irpc/snix backend adapter)
- **Canonical crate/path**: `crates/aspen-blob`, `crates/aspen-castore`, `crates/aspen-cache`
- **Intended audience**: Rust projects that want reusable iroh-backed blob storage, snix castore client/server adapters, or Nix binary cache metadata/signing helpers without the Aspen node app, handler registry, or CLI/web shells.
- **Public API owner**: Aspen storage/cache maintainers
- **Readiness state**: `workspace-internal`
- **Dependency policy class**: runtime-capable reusable storage/cache family

## Package and release metadata

- **Package description**: Content-addressed blob storage over iroh-blobs, snix-compatible castore adapters over irpc, and Nix binary cache narinfo/signing helpers with optional Aspen KV-backed publication.
- **Documentation entrypoint**: Crate-level Rustdoc plus downstream fixtures under the active OpenSpec evidence directory.
- **License policy**: AGPL-3.0-or-later until human license strategy changes.
- **Repository/homepage policy**: Monorepo path until publication policy is decided.
- **Semver/compatibility policy**: No external semver guarantee yet; reusable default APIs become semver-relevant once ready.
- **Publish readiness**: Blocked; do not mark publishable during this change.

## Feature contract

| Crate | Feature set | Status | Purpose |
| --- | --- | --- | --- |
| `aspen-blob` | default | reusable default | Iroh/iroh-blobs-backed blob APIs, background downloads, events, KV-aware blob wrapper, store traits and types. |
| `aspen-blob` | `replication` | Aspen adapter feature | Cluster replication manager, topology watcher, KV replica metadata, and client-RPC blob transfer integration. Enables `aspen-client-api`. |
| `aspen-castore` | default | reusable runtime adapter | snix `BlobService`/`DirectoryService` adapters over irpc/iroh and iroh-blobs-backed server. Circuit-breaker behavior is local to this crate. |
| `aspen-cache` | default | reusable default | Nix narinfo parsing/formatting, NAR dumping, cache metadata types, and Ed25519 signing/verifying helpers. |
| `aspen-cache` | `kv-index` | Aspen adapter feature | `KvCacheIndex` and `ensure_signing_key` integration over Aspen `KeyValueStore` / KV request types. |
| app handlers / node / gateways | explicit consumer features only | compatibility | `aspen-blob-handler`, `aspen-nix-handler`, root `aspen`, bridge/gateway/binary shells stay final consumers and must not leak into reusable defaults. |

## Dependencies

### Internal Aspen dependencies

| Crate | Dependency | Decision | Reason |
| --- | --- | --- | --- |
| `aspen-blob` | `aspen-core` | keep for now | Provides `KeyValueStore` and KV request/response types for `BlobAwareKeyValueStore`; not an app shell. Future seam can move this to alloc-safe trait/type crates. |
| `aspen-blob` | `aspen-client-api` | gated behind `replication` | Only replication pull RPCs need client request/response schemas and `CLIENT_ALPN`. |
| `aspen-castore` | `aspen-blob` | keep | Server wraps a reusable `BlobStore` implementation; this is the storage backend boundary. |
| `aspen-castore` | `aspen-core` / `aspen-core-shell` | removed | Circuit breaker moved local to avoid app/core-shell coupling in the reusable castore graph. |
| `aspen-cache` | `aspen-core`, `aspen-kv-types` | gated behind `kv-index` | KV-backed index and signing-key persistence are Aspen publication adapters, not required for narinfo/signing helpers. |
| `aspen-cache` | `aspen-blob` | removed from default | No reusable metadata/signing code requires blob storage directly. Publication/storage callers own blob integration. |

### External dependencies

| Dependency | Decision | Reason |
| --- | --- | --- |
| `iroh`, `iroh-blobs` | allowed backend exception for `aspen-blob` | They are the explicit blob storage backend purpose. |
| `irpc`, `iroh`, `snix-castore` | allowed backend/adapter exception for `aspen-castore` | Castore's purpose is a concrete irpc/iroh adapter for snix traits. |
| `nix-compat`, `ed25519-dalek`, `sha2`, `data-encoding`, `rand_core_06` | allowed domain dependencies for `aspen-cache` | They implement Nix narinfo, NAR hashing, and cache key signing semantics. |
| `tokio`, `async-trait`, `futures`, `serde`, `serde_json`, `snafu`, `tracing` | keep | Runtime, trait, serialization, error, and observability support for the reusable APIs. |

### Binary/runtime dependencies

No binary, CLI, TUI, web, dogfood, bridge, or gateway crate is allowed in the default reusable graphs. `aspen-nix-cache-gateway`, `aspen-snix-bridge`, root `aspen`, `aspen-cli`, and handlers remain final consumers and appear only in compatibility evidence.

## Compatibility and aliases

- **Old paths**: `aspen_blob::BlobReplicationManager` and related replication types remain available when `aspen-blob/replication` is enabled.
- **New path**: Reusable blob, castore, cache, narinfo, and signing paths stay direct crate imports.
- **Compatibility re-exports**: none. Downstream code should import `aspen-blob`, `aspen-castore`, and `aspen-cache` directly.
- **Feature compatibility**: root `aspen` blob/CI features and `aspen-rpc-core`, `aspen-rpc-handlers`, `aspen-cluster`, and `aspen-nix-handler` features enable the adapter features they require.
- **Removal criteria**: Temporary adapter feature compatibility can be revisited once handler/runtime crates own all app-specific glue.

## Representative consumers

- root `aspen` node setup (`blob`, `ci-core`, `snix` feature bundles)
- `aspen-rpc-core` with `blob`
- `aspen-rpc-handlers` with `blob` and CI/cache features
- `aspen-blob-handler`
- `aspen-cluster` with `blob` / `snix`
- `aspen-snix`
- `aspen-snix-bridge`
- `aspen-nix-cache-gateway`
- `aspen-ci-executor-nix`
- `aspen-nix-handler` with `cache`

## Representative consumers and re-exporters

- **Representative consumers**: root `aspen`, `aspen-rpc-core`, `aspen-rpc-handlers`, `aspen-blob-handler`, `aspen-cluster`, `aspen-snix`, `aspen-snix-bridge`, `aspen-nix-cache-gateway`, `aspen-ci-executor-nix`, `aspen-nix-handler`.
- **Re-exporters**: compatibility re-exports: none. Existing consumers use direct crate paths or feature-enabled dependency paths.

## Dependency exceptions

| candidate | feature_set | dependency_path | owner | reason |
| --- | --- | --- | --- | --- |
| `aspen-blob` | default | `aspen-blob -> iroh` | Aspen storage/cache maintainers | Iroh endpoint/address types are the backend transport purpose for blob storage. |
| `aspen-blob` | default | `aspen-blob -> iroh-blobs` | Aspen storage/cache maintainers | iroh-blobs is the content-addressed storage backend purpose. |
| `aspen-blob` | default | `aspen-blob -> iroh-base` | Aspen storage/cache maintainers | Transitive iroh backend type dependency. |
| `aspen-blob` | default | `aspen-blob -> irpc` | Aspen storage/cache maintainers | Current Aspen KV/core trait path brings irpc transitively until the KV-aware seam is split. |
| `aspen-blob` | default | `aspen-blob -> aspen-core` | Aspen storage/cache maintainers | Current `BlobAwareKeyValueStore` uses Aspen KV traits/types; app-shell split is tracked as a later seam. |
| `aspen-blob` | `replication` | `aspen-blob -> aspen-client-api` | Aspen storage/cache maintainers | Replication pull RPCs are adapter-only compatibility. |
| `aspen-castore` | default | `aspen-castore -> aspen-blob` | Aspen storage/cache maintainers | Castore server wraps the reusable blob store trait/implementation. |
| `aspen-castore` | default | `aspen-castore -> snix-castore` | Aspen storage/cache maintainers | snix trait implementation is the crate purpose. |
| `aspen-castore` | default | `aspen-castore -> irpc` | Aspen storage/cache maintainers | Runtime adapter message framing. |
| `aspen-castore` | default | `aspen-castore -> iroh` | Aspen storage/cache maintainers | Runtime adapter transport. |
| `aspen-cache` | default | `aspen-cache -> nix-compat` | Aspen storage/cache maintainers | Nix narinfo/store path/NAR format support is the crate purpose. |
| `aspen-cache` | default | `aspen-cache -> ed25519-dalek` | Aspen storage/cache maintainers | Nix binary cache signing. |
| `aspen-cache` | default | `aspen-cache -> sha2` | Aspen storage/cache maintainers | NAR hash computation for Nix compatibility. |
| `aspen-cache` | `kv-index` | `aspen-cache -> aspen-core` | Aspen storage/cache maintainers | Optional Aspen KV publication adapter. |
| `aspen-cache` | `kv-index` | `aspen-cache -> aspen-kv-types` | Aspen storage/cache maintainers | Optional Aspen KV request helpers for signing-key persistence. |

## Verification rails

- `cargo check -p aspen-blob --no-default-features`
- `cargo check -p aspen-blob --features replication`
- `cargo tree -p aspen-blob --no-default-features -e normal` plus negative boundary grep for `aspen-client-api`, handlers, root app, and node bootstrap crates
- `cargo test -p aspen-castore circuit_breaker`
- `cargo check -p aspen-castore --no-default-features`
- `cargo tree -p aspen-castore --no-default-features -e normal` plus negative boundary grep for core-shell, handlers, and blob handler crates
- `cargo test -p aspen-cache --no-default-features`
- `cargo check -p aspen-cache --no-default-features`
- `cargo check -p aspen-cache --features kv-index`
- `cargo tree -p aspen-cache --no-default-features -e normal` plus negative boundary grep for Aspen app/runtime/testing crates
- positive downstream fixtures for canonical blob APIs and cache/castore domain APIs
- negative boundary fixtures or metadata checks proving app-only APIs require `replication` / `kv-index`
- compatibility checks for `aspen-rpc-core`, blob handlers, `aspen-snix`, `aspen-snix-bridge`, `aspen-nix-cache-gateway`, and CI/cache executors
- dependency-boundary checker with `--candidate-family blob-castore-cache`, including mutation checks for forbidden app-shell dependencies, undocumented backend exceptions, missing owner, invalid readiness state, and missing downstream fixture evidence

## First-slice status

Current status is `workspace-internal`. I3/I4/I5 have moved the highest-risk app couplings behind features or local helpers:

- `aspen-blob/replication` owns client-RPC replication coupling.
- `aspen-castore` owns its local irpc circuit breaker without `aspen-core-shell`.
- `aspen-cache` default owns narinfo/NAR/signing helpers only; `kv-index` owns Aspen KV publication.

The family is not yet marked `extraction-ready-in-workspace` until downstream fixtures, policy checker updates, and full compatibility evidence are completed.
