# Domain Capability Audit Evidence

Generated: `2026-05-05T00:40:21Z`

## Completed focused slices

This evidence extends the Phase 3 domain-specific capability audit with a focused CI/jobs slice. It does not claim the broad domain-capability task is complete. The remaining list is generated from production `to_operation` code before test modules.

- **Secrets/trust** — domain-specific secrets/transit/PKI operations; generic _secrets: Read/Write negative tests.
- **SNIX store** — SnixRead/SnixWrite mappings and generic snix: negative tests.
- **Net service mesh** — NetPublish/NetUnpublish/NetConnect/NetAdmin mappings; generic /_sys/net/svc/ negative tests.
- **CI pipeline** — CiRead/CiWrite mappings and generic _ci: negative tests.
- **Jobs/workers** — JobsRead/JobsWrite mappings and generic _jobs:/__worker: negative tests.

## CI/jobs slice finding

CI pipeline and job/worker requests previously mapped to generic data operations over `_ci:`, `_jobs:`, and `__worker:` prefixes. They now map to domain-specific operations:

- CI read surfaces -> `Operation::CiRead { resource }`
- CI mutation surfaces -> `Operation::CiWrite { resource }`
- Job/worker read surfaces -> `Operation::JobsRead { resource }`
- Job/worker mutation surfaces -> `Operation::JobsWrite { resource }`

Generic `Capability::Read` / `Capability::Write` over `_ci:`, `_jobs:`, or worker prefixes no longer authorize the audited CI/jobs requests; `Capability::CiRead`, `Capability::CiWrite`, `Capability::JobsRead`, or `Capability::JobsWrite` are required.

## Regression coverage

- `secrets_kv_requests_use_secrets_capabilities_not_generic_prefixes`
- `transit_and_nix_cache_requests_use_transit_capabilities`
- `pki_requests_use_pki_capabilities_not_generic_prefixes`
- `snix_requests_require_snix_scoped_operations`
- `generic_kv_scopes_do_not_authorize_snix_put_operations`
- `net_registry_requests_use_net_capabilities_not_sys_prefixes`
- `generic_sys_prefixes_do_not_authorize_net_registry_requests`
- `ci_requests_require_ci_scoped_operations`
- `generic_ci_prefixes_do_not_authorize_ci_requests`
- `job_requests_use_jobs_capabilities_not_generic_prefixes`
- `generic_jobs_prefixes_do_not_authorize_job_requests`

## Remaining generic internal-domain mappings

These 47 production source handles keep the broad Phase 3 task open for additional focused slices. This pass intentionally re-baselines the list with a stricter generated scanner and does not include test assertions.

- `crates/aspen-client-api/src/messages/to_operation/automerge_ops.rs:16` — `| ClientRpcRequest::AutomergeReceiveSyncMessage { .. } => Some(Some(Operation::Write {`
- `crates/aspen-client-api/src/messages/to_operation/automerge_ops.rs:27` — `| ClientRpcRequest::AutomergeGenerateSyncMessage { .. } => Some(Some(Operation::Read {`
- `crates/aspen-client-api/src/messages/to_operation/blob_ops.rs:16` — `| ClientRpcRequest::DownloadBlobByProvider { .. } => Some(Some(Operation::Write {`
- `crates/aspen-client-api/src/messages/to_operation/blob_ops.rs:26` — `| ClientRpcRequest::GetBlobReplicationStatus { hash } => Some(Some(Operation::Read {`
- `crates/aspen-client-api/src/messages/to_operation/blob_ops.rs:29` — `ClientRpcRequest::ListBlobs { .. } => Some(Some(Operation::Read {`
- `crates/aspen-client-api/src/messages/to_operation/blob_ops.rs:34` — `ClientRpcRequest::BlobReplicatePull { hash, .. } => Some(Some(Operation::Write {`
- `crates/aspen-client-api/src/messages/to_operation/blob_ops.rs:38` — `ClientRpcRequest::TriggerBlobReplication { hash, .. } => Some(Some(Operation::Write {`
- `crates/aspen-client-api/src/messages/to_operation/ci_ops.rs:65` — `Some(Some(Operation::Read {`
- `crates/aspen-client-api/src/messages/to_operation/ci_ops.rs:69` — `ClientRpcRequest::CacheStats => Some(Some(Operation::Read {`
- `crates/aspen-client-api/src/messages/to_operation/ci_ops.rs:72` — `ClientRpcRequest::NixCacheGetPublicKey => Some(Some(Operation::Read {`
- `crates/aspen-client-api/src/messages/to_operation/ci_ops.rs:84` — `Some(Some(Operation::Read {`
- `crates/aspen-client-api/src/messages/to_operation/coordination_ops.rs:29` — `| ClientRpcRequest::LockRenew { key, .. } => Some(Some(Operation::Write {`
- `crates/aspen-client-api/src/messages/to_operation/coordination_ops.rs:35` — `Some(Operation::Write {`
- `crates/aspen-client-api/src/messages/to_operation/coordination_ops.rs:44` — `Some(Operation::Write {`
- `crates/aspen-client-api/src/messages/to_operation/coordination_ops.rs:53` — `| ClientRpcRequest::SequenceCurrent { key } => Some(Some(Operation::Read {`
- `crates/aspen-client-api/src/messages/to_operation/coordination_ops.rs:65` — `| ClientRpcRequest::SequenceReserve { key, .. } => Some(Some(Operation::Write {`
- `crates/aspen-client-api/src/messages/to_operation/coordination_ops.rs:78` — `| ClientRpcRequest::RateLimiterReset { key, .. } => Some(Some(Operation::Write {`
- `crates/aspen-client-api/src/messages/to_operation/coordination_ops.rs:82` — `ClientRpcRequest::RateLimiterAvailable { key, .. } => Some(Some(Operation::Read {`
- `crates/aspen-client-api/src/messages/to_operation/coordination_ops.rs:87` — `Some(Some(Operation::Write {`
- `crates/aspen-client-api/src/messages/to_operation/coordination_ops.rs:92` — `ClientRpcRequest::BarrierStatus { name } => Some(Some(Operation::Read {`
- `crates/aspen-client-api/src/messages/to_operation/coordination_ops.rs:104` — `| ClientRpcRequest::SemaphoreRelease { name, .. } => Some(Some(Operation::Write {`
- `crates/aspen-client-api/src/messages/to_operation/coordination_ops.rs:108` — `ClientRpcRequest::SemaphoreStatus { name } => Some(Some(Operation::Read {`
- `crates/aspen-client-api/src/messages/to_operation/coordination_ops.rs:118` — `| ClientRpcRequest::RWLockDowngrade { name, .. } => Some(Some(Operation::Write {`
- `crates/aspen-client-api/src/messages/to_operation/coordination_ops.rs:122` — `ClientRpcRequest::RWLockStatus { name } => Some(Some(Operation::Read {`
- `crates/aspen-client-api/src/messages/to_operation/coordination_ops.rs:141` — `| ClientRpcRequest::QueueRedriveDLQ { queue_name, .. } => Some(Some(Operation::Write {`
- `crates/aspen-client-api/src/messages/to_operation/coordination_ops.rs:147` — `| ClientRpcRequest::QueueGetDLQ { queue_name, .. } => Some(Some(Operation::Read {`
- `crates/aspen-client-api/src/messages/to_operation/coordination_ops.rs:161` — `| ClientRpcRequest::ServiceUpdateMetadata { service_name, .. } => Some(Some(Operation::Write {`
- `crates/aspen-client-api/src/messages/to_operation/coordination_ops.rs:166` — `| ClientRpcRequest::ServiceGetInstance { service_name, .. } => Some(Some(Operation::Read {`
- `crates/aspen-client-api/src/messages/to_operation/coordination_ops.rs:169` — `ClientRpcRequest::ServiceList { prefix, .. } => Some(Some(Operation::Read {`
- `crates/aspen-client-api/src/messages/to_operation/docs_ops.rs:10` — `ClientRpcRequest::DocsSet { key, value } => Some(Some(Operation::Write {`
- `crates/aspen-client-api/src/messages/to_operation/docs_ops.rs:14` — `ClientRpcRequest::DocsGet { key } | ClientRpcRequest::DocsDelete { key } => Some(Some(Operation::Read {`
- `crates/aspen-client-api/src/messages/to_operation/docs_ops.rs:17` — `ClientRpcRequest::DocsList { .. } | ClientRpcRequest::DocsStatus => Some(Some(Operation::Read {`
- `crates/aspen-client-api/src/messages/to_operation/hooks_ops.rs:10` — `ClientRpcRequest::HookList | ClientRpcRequest::HookGetMetrics { .. } => Some(Some(Operation::Read {`
- `crates/aspen-client-api/src/messages/to_operation/hooks_ops.rs:13` — `ClientRpcRequest::HookTrigger { .. } => Some(Some(Operation::Write {`
- `crates/aspen-client-api/src/messages/to_operation/kv_ops.rs:20` — `ClientRpcRequest::GetVaultKeys { vault_name: key } => Some(Some(Operation::Read { key: key.clone() })),`
- `crates/aspen-client-api/src/messages/to_operation/kv_ops.rs:32` — `ClientRpcRequest::IndexScan { .. } | ClientRpcRequest::IndexList => Some(Some(Operation::Read {`
- `crates/aspen-client-api/src/messages/to_operation/kv_ops.rs:38` — `Some(Some(Operation::Write {`
- `crates/aspen-client-api/src/messages/to_operation/lease_ops.rs:12` — `| ClientRpcRequest::LeaseKeepalive { .. } => Some(Some(Operation::Write {`
- `crates/aspen-client-api/src/messages/to_operation/lease_ops.rs:16` — `ClientRpcRequest::LeaseTimeToLive { .. } | ClientRpcRequest::LeaseList => Some(Some(Operation::Read {`
- `crates/aspen-client-api/src/messages/to_operation/observability_ops.rs:10` — `ClientRpcRequest::TraceIngest { .. } => Some(Some(Operation::Write {`
- `crates/aspen-client-api/src/messages/to_operation/observability_ops.rs:17` — `| ClientRpcRequest::TraceSearch { .. } => Some(Some(Operation::Read {`
- `crates/aspen-client-api/src/messages/to_operation/observability_ops.rs:21` — `ClientRpcRequest::MetricIngest { .. } => Some(Some(Operation::Write {`
- `crates/aspen-client-api/src/messages/to_operation/observability_ops.rs:26` — `ClientRpcRequest::MetricList { .. } | ClientRpcRequest::MetricQuery { .. } => Some(Some(Operation::Read {`
- `crates/aspen-client-api/src/messages/to_operation/observability_ops.rs:30` — `ClientRpcRequest::AlertCreate { .. } | ClientRpcRequest::AlertDelete { .. } => Some(Some(Operation::Write {`
- `crates/aspen-client-api/src/messages/to_operation/observability_ops.rs:35` — `ClientRpcRequest::AlertEvaluate { .. } => Some(Some(Operation::Write {`
- `crates/aspen-client-api/src/messages/to_operation/observability_ops.rs:40` — `ClientRpcRequest::AlertList | ClientRpcRequest::AlertGet { .. } => Some(Some(Operation::Read {`
- `crates/aspen-client-api/src/messages/to_operation/sql_ops.rs:10` — `ClientRpcRequest::ExecuteSql { .. } => Some(Some(Operation::Read {`

## Verification commands

- `cargo test -p aspen-client-api --features auth to_operation -- --nocapture`
- `cargo check -p aspen-client-api --features auth`
- `scripts/tigerstyle-check.sh`
- `openspec validate full-aspen-hardening-audit --strict --json`
- `python ~/.hermes/skills/agentkit-port/openspec/scripts/openspec_helper.py verify full-aspen-hardening-audit --json || true`
