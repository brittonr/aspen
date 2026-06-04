## Phase 1: Canonical sync records

- [x] [serial] r[molten.job_dag_remote_sync.request_dto] Define `job-sync-request-v1` with job ref, selected stages, target peer, policy refs, capability refs, evidence refs, and no-execution checks.
- [x] [serial] r[molten.job_dag_remote_sync.plan_dto] Define `job-sync-plan-v1` with source roots, dependency closure, target missing set, selected stages, and closure checks.
- [x] [serial] r[molten.job_dag_remote_sync.receipt_dto] Define `job-sync-receipt-v1` for sync-plan and sync-loopback operations.

## Phase 2: Loopback closure sync

- [x] [serial] r[molten.job_dag_remote_sync.closure_roots] Resolve source job artifact refs and selected stage artifact refs as closure roots.
- [x] [serial] r[molten.job_dag_remote_sync.missing_set] Compute target missing sets from full artifact refs without using names, paths, or mtimes.
- [x] [serial] r[molten.job_dag_remote_sync.loopback_install] Install missing artifacts into a target registry in dependency-first order.
- [x] [parallel] r[molten.job_dag_remote_sync.hash_verify] Verify source and target canonical artifact envelopes before accepting sync.
- [x] [parallel] r[molten.job_dag_remote_sync.no_execution] Keep sync separate from execution and emit no-execution checks.

## Phase 3: CLI and tests

- [x] [serial] r[molten.job_dag_remote_sync.cli] Add `molten test job sync-plan` and `sync-loopback` commands.
- [x] [parallel] r[molten.job_dag_remote_sync.ledger_classification] Classify sync requests, plans, and receipts in the local ledger/catalog surface.
- [x] [parallel] r[molten.job_dag_remote_sync.tests] Add tests for empty target sync, repeated no-op sync, closure dependency install, and no-execution receipt checks.
