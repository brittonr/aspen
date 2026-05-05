# Domain capability audit evidence

Generated: 2026-05-05T01:06:02Z

## Completed focused slices

- **Secrets/trust**: domain-specific secrets/transit/PKI operations; generic _secrets: Read/Write negative tests
- **SNIX store**: SnixRead/SnixWrite mappings and generic snix: negative tests
- **Net service mesh**: NetPublish/NetUnpublish/NetConnect/NetAdmin mappings; generic /_sys/net/svc/ negative tests
- **CI pipeline**: CiRead/CiWrite mappings and generic _ci: negative tests
- **Jobs/workers**: JobsRead/JobsWrite mappings and generic _jobs:/__worker: negative tests
- **Blob/docs/hooks**: BlobRead/BlobWrite, DocsRead/DocsWrite, and HooksRead/HooksWrite mappings; generic _blob:/_docs:/_hooks: negative tests
- **Coordination primitives and leases**: CoordinationRead/CoordinationWrite mappings and generic coordination-prefix negative tests for queues and leases

## Current slice: coordination primitives and leases

Converted coordination and lease request authorization from generic internal storage prefixes to domain-specific `Operation::CoordinationRead` / `Operation::CoordinationWrite` with scoped `Capability::CoordinationRead` / `Capability::CoordinationWrite`.

Generic data capabilities over `_queue:` and `_lease:` are covered by negative regressions and no longer authorize the audited requests. Root token generation now includes broad coordination read/write capabilities so root remains able to authorize the domain-specific operations.

## Remaining generic internal-domain mappings

Remaining count: 17

- `crates/aspen-client-api/src/messages/to_operation/observability_ops.rs`: 7
- `crates/aspen-client-api/src/messages/to_operation/ci_ops.rs`: 4
- `crates/aspen-client-api/src/messages/to_operation/kv_ops.rs`: 3
- `crates/aspen-client-api/src/messages/to_operation/automerge_ops.rs`: 2
- `crates/aspen-client-api/src/messages/to_operation/sql_ops.rs`: 1

## Remaining handles

- `crates/aspen-client-api/src/messages/to_operation/automerge_ops.rs:16` — `| ClientRpcRequest::AutomergeReceiveSyncMessage { .. } => Some(Some(Operation::Write {`
- `crates/aspen-client-api/src/messages/to_operation/automerge_ops.rs:27` — `| ClientRpcRequest::AutomergeGenerateSyncMessage { .. } => Some(Some(Operation::Read {`
- `crates/aspen-client-api/src/messages/to_operation/ci_ops.rs:65` — `Some(Some(Operation::Read {`
- `crates/aspen-client-api/src/messages/to_operation/ci_ops.rs:69` — `ClientRpcRequest::CacheStats => Some(Some(Operation::Read {`
- `crates/aspen-client-api/src/messages/to_operation/ci_ops.rs:72` — `ClientRpcRequest::NixCacheGetPublicKey => Some(Some(Operation::Read {`
- `crates/aspen-client-api/src/messages/to_operation/ci_ops.rs:84` — `Some(Some(Operation::Read {`
- `crates/aspen-client-api/src/messages/to_operation/kv_ops.rs:20` — `ClientRpcRequest::GetVaultKeys { vault_name: key } => Some(Some(Operation::Read { key: key.clone() })),`
- `crates/aspen-client-api/src/messages/to_operation/kv_ops.rs:32` — `ClientRpcRequest::IndexScan { .. } | ClientRpcRequest::IndexList => Some(Some(Operation::Read {`
- `crates/aspen-client-api/src/messages/to_operation/kv_ops.rs:38` — `Some(Some(Operation::Write {`
- `crates/aspen-client-api/src/messages/to_operation/observability_ops.rs:10` — `ClientRpcRequest::TraceIngest { .. } => Some(Some(Operation::Write {`
- `crates/aspen-client-api/src/messages/to_operation/observability_ops.rs:17` — `| ClientRpcRequest::TraceSearch { .. } => Some(Some(Operation::Read {`
- `crates/aspen-client-api/src/messages/to_operation/observability_ops.rs:21` — `ClientRpcRequest::MetricIngest { .. } => Some(Some(Operation::Write {`
- `crates/aspen-client-api/src/messages/to_operation/observability_ops.rs:26` — `ClientRpcRequest::MetricList { .. } | ClientRpcRequest::MetricQuery { .. } => Some(Some(Operation::Read {`
- `crates/aspen-client-api/src/messages/to_operation/observability_ops.rs:30` — `ClientRpcRequest::AlertCreate { .. } | ClientRpcRequest::AlertDelete { .. } => Some(Some(Operation::Write {`
- `crates/aspen-client-api/src/messages/to_operation/observability_ops.rs:35` — `ClientRpcRequest::AlertEvaluate { .. } => Some(Some(Operation::Write {`
- `crates/aspen-client-api/src/messages/to_operation/observability_ops.rs:40` — `ClientRpcRequest::AlertList | ClientRpcRequest::AlertGet { .. } => Some(Some(Operation::Read {`
- `crates/aspen-client-api/src/messages/to_operation/sql_ops.rs:10` — `ClientRpcRequest::ExecuteSql { .. } => Some(Some(Operation::Read {`
