# Reserved Storage Prefix Audit

Generated: `2026-05-04T22:40:13Z`

## Scope

Phase 3 reserved storage prefix audit for generic KV read/write/scan/watch/batch/conditional operations.

This slice audits only generic client KV authority paths. It does not claim that every domain API has already moved to domain-specific capability variants; that remains the next Phase 3 task.

## Reserved internal prefixes

- `_automerge:`
- `_barrier:`
- `_blob:`
- `_cache:`
- `_ci:`
- `_counter:`
- `_docs:`
- `_forge:`
- `_hooks:`
- `_jobs:`
- `_lease:`
- `_lock:`
- `_queue:`
- `_ratelimit:`
- `_rwlock:`
- `_secrets:`
- `_semaphore:`
- `_service:`
- `_sql:`
- `_sys:`
- `__worker:`
- `/_sys/`

SNIX store keys (`snix:dir:`, `snix:pathinfo:`) remain separately mapped to SNIX-specific read/write operations for exact generic KV reads/writes and to cluster-admin for broad overlapping scans.

## Verified boundaries

- `reserved-prefix-registry` — `crates/aspen-client-api/src/messages/to_operation/mod.rs:33`: `const RESERVED_INTERNAL_KEY_PREFIXES: &[&str] = &[`
- `generic-read-write-guard` — `crates/aspen-client-api/src/messages/to_operation/mod.rs:90`: `fn snix_operation_for_read_key(key: &str) -> Operation {`
- `generic-scan-watch-overlap-guard` — `crates/aspen-client-api/src/messages/to_operation/mod.rs:99`: `fn scan_operation_for_prefix(prefix: &str) -> Operation {`
- `generic-write-guard` — `crates/aspen-client-api/src/messages/to_operation/mod.rs:117`: `fn snix_operation_for_write_key(key: &str, value: Vec<u8>) -> Operation {`
- `batch-reserved-key-guard` — `crates/aspen-client-api/src/messages/to_operation/batch_ops.rs:38`: `fn batch_operation_for_read_keys(keys: &[alloc::string::String]) -> Option<Operation> {`
- `conditional-batch-condition-inclusion` — `crates/aspen-client-api/src/messages/to_operation/batch_ops.rs:27`: `fn conditional_batch_write_keys(`
- `watch-uses-scan-guard` — `crates/aspen-client-api/src/messages/to_operation/watch_ops.rs:9`: `ClientRpcRequest::WatchCreate { prefix, .. } => Some(Some(scan_operation_for_prefix(prefix))),`
- `generic-capability-admin-only` — `crates/aspen-auth-core/src/capability.rs:411`: `(Capability::ClusterAdmin, Operation::ClusterAdmin { .. }) => Some(true),`

## Negative regression tests

- `generic_kv_operations_over_internal_prefixes_require_admin` — `crates/aspen-client-api/src/messages/to_operation/kv_ops.rs:117`
- `broad_scans_overlapping_internal_namespaces_require_admin` — `crates/aspen-client-api/src/messages/to_operation/kv_ops.rs:158`
- `internal_keys_in_batch_operations_require_admin` — `crates/aspen-client-api/src/messages/to_operation/batch_ops.rs:175`
- `internal_condition_keys_in_conditional_batch_require_admin` — `crates/aspen-client-api/src/messages/to_operation/batch_ops.rs:203`
- `internal_watch_prefix_requires_admin` — `crates/aspen-client-api/src/messages/to_operation/watch_ops.rs:50`

## Finding and disposition

Concrete bypass class fixed: generic KV read/write/hash/CAS/delete, scan/watch, batch read/write, and conditional batch condition keys over Aspen internal prefixes now fail closed to ClusterAdmin instead of being authorizable by broad generic data capabilities.

Generic broad `Read`, `Write`, `Watch`, and `Full` capabilities over an empty prefix no longer authorize the audited internal-prefix requests; only `Capability::ClusterAdmin` authorizes the new `reserved_internal_*` fail-closed operations.

## Remaining scope

Domain APIs still map some internal prefixes to generic Operation::Read/Write; that is intentionally left for the next OpenSpec task on domain-specific capabilities.
