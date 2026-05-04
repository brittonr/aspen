# Phase 3 Domain-Specific Capability Audit — Secrets/Trust Slice

Generated: `2026-05-04T23:39:02Z`

## Scope

Audits and remediates the highest-risk remaining domain-specific capability seam: secrets/trust request authorization. It also records cross-domain status so the umbrella task remains honest about follow-up work.

## Finding

Concrete bypass class fixed for secrets/trust: ClientRpcRequest and standalone SecretsRequest mappings no longer authorize secrets, transit, PKI, or Nix-cache signing operations through generic _secrets: Read/Write prefixes. Generic data capabilities do not authorize the new domain-specific operations; Secrets/Transit/PKI capabilities do.

## Verified source handles

- `remediated` `crates/aspen-client-api/src/messages/to_operation/secrets_ops.rs:24` — `Operation::SecretsRead {` — Client RPC KV secret reads now map to domain-specific SecretsRead.
- `remediated` `crates/aspen-client-api/src/messages/to_operation/secrets_ops.rs:80` — `ClientRpcRequest::SecretsTransitEncrypt { mount, name, .. } => Some(Some(Operation::TransitEncrypt {` — Transit encrypt uses TransitEncrypt with mount-scoped key name.
- `remediated` `crates/aspen-client-api/src/messages/to_operation/secrets_ops.rs:114` — `ClientRpcRequest::SecretsPkiIssue { mount, role, .. } => Some(Some(Operation::PkiIssue {` — PKI issuance uses PkiIssue with mount-scoped role.
- `remediated` `crates/aspen-client-api/src/messages/to_operation/secrets_ops.rs:77` — `| ClientRpcRequest::SecretsTransitRotateKey { mount, name } => Some(Some(Operation::TransitKeyManage {` — Nix-cache key create/rotate/delete use TransitKeyManage.
- `remediated` `crates/aspen-client-api/src/messages/secrets.rs:181` — `Some(Operation::SecretsRead {` — Standalone SecretsRequest mapping matches domain-specific ClientRpcRequest mapping.
- `existing-domain-capability` `crates/aspen-auth-core/src/capability.rs:437` — `fn authorizes_secrets(&self, op: &Operation) -> Option<bool> {` — Secrets capabilities are distinct from generic data capabilities.
- `existing-domain-capability` `crates/aspen-auth-core/src/capability.rs:484` — `fn authorizes_transit(&self, op: &Operation) -> Option<bool> {` — Transit capabilities are distinct from generic data capabilities.
- `existing-domain-capability` `crates/aspen-auth-core/src/capability.rs:505` — `fn authorizes_pki(&self, op: &Operation) -> Option<bool> {` — PKI capabilities are distinct from generic data capabilities.
- `already-domain-specific` `crates/aspen-client-api/src/messages/to_operation/ci_ops.rs:81` — `ClientRpcRequest::SnixDirectoryGet { digest } => Some(Some(Operation::SnixRead {` — SNIX CI endpoints already use SnixRead/SnixWrite for store authority.
- `already-domain-specific` `crates/aspen-client-api/src/messages/to_operation/forge_ops.rs:38` — `Operation::FederationPull { fed_id: fed_id.into() }` — Federation pull/push endpoints already use federation-specific operations.
- `already-admin-specific` `crates/aspen-client-api/src/messages/to_operation/deploy_ops.rs:14` — `| ClientRpcRequest::ClusterDeployStatus => Some(Some(Operation::ClusterAdmin {` — Deploy control-plane endpoints map to ClusterAdmin.

## Remaining generic internal-domain mappings

These are intentionally not claimed fixed by this slice and keep the broad Phase 3 task open for follow-up domain-capability changes.

- `crates/aspen-client-api/src/messages/to_operation/automerge_ops.rs:16` — `| ClientRpcRequest::AutomergeReceiveSyncMessage { .. } => Some(Some(Operation::Write {`
- `crates/aspen-client-api/src/messages/to_operation/automerge_ops.rs:27` — `| ClientRpcRequest::AutomergeGenerateSyncMessage { .. } => Some(Some(Operation::Read {`
- `crates/aspen-client-api/src/messages/to_operation/blob_ops.rs:16` — `| ClientRpcRequest::DownloadBlobByProvider { .. } => Some(Some(Operation::Write {`
- `crates/aspen-client-api/src/messages/to_operation/blob_ops.rs:26` — `| ClientRpcRequest::GetBlobReplicationStatus { hash } => Some(Some(Operation::Read {`
- `crates/aspen-client-api/src/messages/to_operation/blob_ops.rs:29` — `ClientRpcRequest::ListBlobs { .. } => Some(Some(Operation::Read {`
- `crates/aspen-client-api/src/messages/to_operation/blob_ops.rs:34` — `ClientRpcRequest::BlobReplicatePull { hash, .. } => Some(Some(Operation::Write {`
- `crates/aspen-client-api/src/messages/to_operation/blob_ops.rs:38` — `ClientRpcRequest::TriggerBlobReplication { hash, .. } => Some(Some(Operation::Write {`
- `crates/aspen-client-api/src/messages/to_operation/ci_ops.rs:14` — `Some(Some(Operation::Read {`
- `crates/aspen-client-api/src/messages/to_operation/ci_ops.rs:18` — `ClientRpcRequest::CiGetRefStatus { repo_id, ref_name } => Some(Some(Operation::Read {`
- `crates/aspen-client-api/src/messages/to_operation/ci_ops.rs:21` — `ClientRpcRequest::CiListRuns { repo_id, .. } => Some(Some(Operation::Read {`
- `crates/aspen-client-api/src/messages/to_operation/ci_ops.rs:26` — `| ClientRpcRequest::CiUnwatchRepo { repo_id } => Some(Some(Operation::Write {`
- `crates/aspen-client-api/src/messages/to_operation/ci_ops.rs:30` — `ClientRpcRequest::CiCancelRun { run_id, .. } => Some(Some(Operation::Write {`
- `crates/aspen-client-api/src/messages/to_operation/ci_ops.rs:34` — `ClientRpcRequest::CiListArtifacts { job_id, run_id } => Some(Some(Operation::Read {`
- `crates/aspen-client-api/src/messages/to_operation/ci_ops.rs:37` — `ClientRpcRequest::CiGetArtifact { blob_hash } => Some(Some(Operation::Read {`
- `crates/aspen-client-api/src/messages/to_operation/ci_ops.rs:40` — `ClientRpcRequest::CiGetJobLogs { run_id, job_id, .. } => Some(Some(Operation::Read {`
- `crates/aspen-client-api/src/messages/to_operation/ci_ops.rs:43` — `ClientRpcRequest::CiSubscribeLogs { run_id, job_id, .. } => Some(Some(Operation::Read {`
- `crates/aspen-client-api/src/messages/to_operation/ci_ops.rs:46` — `ClientRpcRequest::CiGetJobOutput { run_id, job_id } => Some(Some(Operation::Read {`
- `crates/aspen-client-api/src/messages/to_operation/ci_ops.rs:64` — `ClientRpcRequest::NixCacheGetPublicKey => Some(Some(Operation::Read {`
- `crates/aspen-client-api/src/messages/to_operation/coordination_ops.rs:29` — `| ClientRpcRequest::LockRenew { key, .. } => Some(Some(Operation::Write {`
- `crates/aspen-client-api/src/messages/to_operation/coordination_ops.rs:35` — `Some(Operation::Write {`
- `crates/aspen-client-api/src/messages/to_operation/coordination_ops.rs:44` — `Some(Operation::Write {`
- `crates/aspen-client-api/src/messages/to_operation/coordination_ops.rs:53` — `| ClientRpcRequest::SequenceCurrent { key } => Some(Some(Operation::Read {`
- `crates/aspen-client-api/src/messages/to_operation/coordination_ops.rs:65` — `| ClientRpcRequest::SequenceReserve { key, .. } => Some(Some(Operation::Write {`
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
- `crates/aspen-client-api/src/messages/to_operation/jobs_ops.rs:15` — `| ClientRpcRequest::WorkerDeregister { .. } => Some(Some(Operation::Write {`
- `crates/aspen-client-api/src/messages/to_operation/jobs_ops.rs:24` — `| ClientRpcRequest::WorkerStatus => Some(Some(Operation::Read {`
- `crates/aspen-client-api/src/messages/to_operation/kv_ops.rs:32` — `ClientRpcRequest::IndexScan { .. } | ClientRpcRequest::IndexList => Some(Some(Operation::Read {`
- `crates/aspen-client-api/src/messages/to_operation/kv_ops.rs:38` — `Some(Some(Operation::Write {`
- `crates/aspen-client-api/src/messages/to_operation/lease_ops.rs:12` — `| ClientRpcRequest::LeaseKeepalive { .. } => Some(Some(Operation::Write {`
- `crates/aspen-client-api/src/messages/to_operation/lease_ops.rs:16` — `ClientRpcRequest::LeaseTimeToLive { .. } | ClientRpcRequest::LeaseList => Some(Some(Operation::Read {`
- `crates/aspen-client-api/src/messages/to_operation/net_ops.rs:9` — `ClientRpcRequest::NetPublish { name, .. } => Some(Some(Operation::Write {`
- `crates/aspen-client-api/src/messages/to_operation/net_ops.rs:13` — `ClientRpcRequest::NetUnpublish { name, .. } => Some(Some(Operation::Write {`
- `crates/aspen-client-api/src/messages/to_operation/net_ops.rs:17` — `ClientRpcRequest::NetLookup { name, .. } => Some(Some(Operation::Read {`
- `crates/aspen-client-api/src/messages/to_operation/net_ops.rs:20` — `ClientRpcRequest::NetList { .. } => Some(Some(Operation::Read {`
- `crates/aspen-client-api/src/messages/to_operation/observability_ops.rs:10` — `ClientRpcRequest::TraceIngest { .. } => Some(Some(Operation::Write {`
- `crates/aspen-client-api/src/messages/to_operation/observability_ops.rs:17` — `| ClientRpcRequest::TraceSearch { .. } => Some(Some(Operation::Read {`
- `crates/aspen-client-api/src/messages/to_operation/observability_ops.rs:21` — `ClientRpcRequest::MetricIngest { .. } => Some(Some(Operation::Write {`
- `crates/aspen-client-api/src/messages/to_operation/observability_ops.rs:26` — `ClientRpcRequest::MetricList { .. } | ClientRpcRequest::MetricQuery { .. } => Some(Some(Operation::Read {`
- `crates/aspen-client-api/src/messages/to_operation/observability_ops.rs:30` — `ClientRpcRequest::AlertCreate { .. } | ClientRpcRequest::AlertDelete { .. } => Some(Some(Operation::Write {`
- `crates/aspen-client-api/src/messages/to_operation/observability_ops.rs:35` — `ClientRpcRequest::AlertEvaluate { .. } => Some(Some(Operation::Write {`
- `crates/aspen-client-api/src/messages/to_operation/observability_ops.rs:40` — `ClientRpcRequest::AlertList | ClientRpcRequest::AlertGet { .. } => Some(Some(Operation::Read {`
- `crates/aspen-client-api/src/messages/to_operation/sql_ops.rs:10` — `ClientRpcRequest::ExecuteSql { .. } => Some(Some(Operation::Read {`

## Regression coverage

- `secrets_kv_requests_use_secrets_capabilities_not_generic_prefixes`
- `transit_and_nix_cache_requests_use_transit_capabilities`
- `pki_requests_use_pki_capabilities_not_generic_prefixes`

## Verification commands

- `cargo test -p aspen-client-api --features auth secrets_ -- --nocapture`
- `cargo test -p aspen-client-api --features auth to_operation -- --nocapture`
- `cargo check -p aspen-client-api --features auth`
- `scripts/tigerstyle-check.sh`
- `openspec validate full-aspen-hardening-audit --strict --json`
- `python ~/.hermes/skills/agentkit-port/openspec/scripts/openspec_helper.py verify full-aspen-hardening-audit --json || true`
- `git diff --check`
