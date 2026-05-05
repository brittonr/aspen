# Domain Capability Audit Evidence

Generated: 2026-05-05T01:29:22Z

## Summary

Phase 3 has converted the highest-risk internal-domain request families from generic data-prefix authorization to domain-specific capabilities in focused verified slices. The latest slice converted Nix binary-cache metadata and migration status reads to `CacheRead`.

## Completed focused slices

- **Secrets/trust** — domain-specific secrets/transit/PKI operations; generic _secrets: Read/Write negative tests.
- **SNIX store** — SnixRead/SnixWrite mappings and generic snix: negative tests.
- **Net service mesh** — NetPublish/NetUnpublish/NetConnect/NetAdmin mappings; generic /_sys/net/svc/ negative tests.
- **CI pipeline** — CiRead/CiWrite mappings and generic _ci: negative tests.
- **Jobs/workers** — JobsRead/JobsWrite mappings and generic _jobs:/__worker: negative tests.
- **Blob/docs/hooks** — BlobRead/BlobWrite, DocsRead/DocsWrite, and HooksRead/HooksWrite mappings; generic _blob:/_docs:/_hooks: negative tests.
- **Coordination primitives and leases** — CoordinationRead/CoordinationWrite mappings and generic coordination-prefix negative tests for queues and leases.
- **Observability** — ObservabilityRead/ObservabilityWrite mappings for traces, metrics, and alerts; generic _sys:metrics: negative tests.
- **Nix binary cache** — CacheRead mappings for narinfo lookup/download, cache stats, public signing key, and migration status/validation; generic _cache:/_sys:nix-cache: negative tests.

## Nix binary cache slice

- Converted cache narinfo lookup/download, cache stats, public signing key lookup, and cache migration status/validation away from `_cache:*` / `_sys:nix-cache:*` generic `Read` operations.
- Added `Capability::CacheRead` / `Capability::CacheWrite` and matching `Operation` variants with prefix containment for delegation.
- Updated root token generation to include broad cache read/write capabilities.
- Added regressions `cache_requests_require_cache_scoped_operations` and `generic_cache_prefixes_do_not_authorize_cache_requests`.

## Remaining generic internal-domain mappings

Remaining count: **6**

- `automerge_ops.rs`: 2
- `kv_ops.rs`: 3
- `sql_ops.rs`: 1

### Remaining mapping handles

- `crates/aspen-client-api/src/messages/to_operation/automerge_ops.rs:16` — `| ClientRpcRequest::AutomergeReceiveSyncMessage { .. } => Some(Some(Operation::Write {`
- `crates/aspen-client-api/src/messages/to_operation/automerge_ops.rs:27` — `| ClientRpcRequest::AutomergeGenerateSyncMessage { .. } => Some(Some(Operation::Read {`
- `crates/aspen-client-api/src/messages/to_operation/kv_ops.rs:20` — `ClientRpcRequest::GetVaultKeys { vault_name: key } => Some(Some(Operation::Read { key: key.clone() })),`
- `crates/aspen-client-api/src/messages/to_operation/kv_ops.rs:32` — `ClientRpcRequest::IndexScan { .. } | ClientRpcRequest::IndexList => Some(Some(Operation::Read {`
- `crates/aspen-client-api/src/messages/to_operation/kv_ops.rs:38` — `Some(Some(Operation::Write {`
- `crates/aspen-client-api/src/messages/to_operation/sql_ops.rs:10` — `ClientRpcRequest::ExecuteSql { .. } => Some(Some(Operation::Read {`
