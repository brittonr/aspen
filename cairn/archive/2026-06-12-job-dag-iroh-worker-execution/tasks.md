## Phase 0: Prerequisite binding

- [x] [serial] r[molten.job_dag_iroh_worker.spec.target_admission] Require loopback-compatible execution request/receipt verification before worker execution runs.
- [x] [parallel] r[molten.job_dag_iroh_worker.spec.transport_not_authority] Reuse sync, admission, capability, authority, resource, and evidence refs instead of inventing a second authority path.

## Phase 1: Worker records

- [x] [serial] r[molten.job_dag_iroh_worker.spec.target_admission] Define `job-worker-request-v1` with job, target peer, selected stages, sync, admission, execution request, authority, resource, peer-bootstrap, node-identity, and evidence refs.
- [x] [serial] r[molten.job_dag_iroh_worker.spec.replay] Define assignment/status/result/worker receipt DTOs with execution receipts, outputs, resources, delivery logs, diagnostics, and checks.
- [x] [parallel] r[molten.job_dag_iroh_worker.spec.replay] Classify worker artifacts in ledger/catalog/MCP views.
- [x] [parallel] r[molten.job_dag_iroh_worker.spec.target_admission] Deny raw closures, source paths, unverified executable artifacts, missing stage artifact refs, and source-registry arguments before stage execution.

## Phase 2: Transport and target verification

- [x] [serial] r[molten.job_dag_iroh_worker.spec.transport_not_authority] Carry worker request/status/result records over remote dataspace local-gossip envelopes with transport evidence distinct from authority.
- [x] [serial] r[molten.job_dag_iroh_worker.spec.target_admission] On target, verify sync/admission/execution request refs before running any stage.
- [x] [parallel] r[molten.job_dag_iroh_worker.spec.target_admission] Execute using target registry/storage/cache/chunk roots only and deny source registry arguments.
- [x] [parallel] r[molten.job_dag_iroh_worker.spec.transport_not_authority] Bind peer bootstrap, node identity, authority, and resource evidence before execution.

## Phase 3: Recorded replay and live diagnostics

- [x] [serial] r[molten.job_dag_iroh_worker.spec.replay] Record worker request/status/result delivery logs and execution receipts for deterministic gates.
- [x] [serial] r[molten.job_dag_iroh_worker.spec.replay] Add live Iroh-shaped worker mode marked non-replayable unless a complete delivery/effect log is captured.
- [x] [parallel] r[molten.job_dag_iroh_worker.spec.replay] Import worker result receipts, output refs, status records, and stage receipts into the evidence ledger.
- [x] [parallel] r[molten.job_dag_iroh_worker.spec.transport_not_authority] Bind resource consumption/accounting refs to worker status/result records.

## Phase 4: Tests

- [x] [serial] r[molten.job_dag_iroh_worker.spec.replay] Add deterministic two-peer local-gossip worker test from sync through result import.
- [x] [serial] r[molten.job_dag_iroh_worker.spec.target_admission] Test missing admission, denied admission, stale sync, target mismatch, missing artifact, and unrecorded-live gate denial.
- [x] [parallel] r[molten.job_dag_iroh_worker.spec.target_admission] Assert target remote outputs equal equivalent local execution for supported deterministic stages.
- [x] [parallel] r[molten.job_dag_iroh_worker.spec.replay] Add Hegel properties for worker request identity, recorded replay evidence, and no-source-state invariant.
