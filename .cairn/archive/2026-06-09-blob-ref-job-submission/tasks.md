## Phase 1: Job ref model

- [x] [serial] r[molten.blob_ref_jobs.payload_model] Define job submission DTOs with job id, operation id, executable/content refs, size/format hints, schema refs, effect manifest refs, handler profile, authority context, and evidence refs.
- [x] [serial] r[molten.blob_ref_jobs.no_inline_large_bytes] Enforce content refs for large executables, inputs, outputs, logs, and datasets.
- [x] [parallel] r[molten.blob_ref_jobs.status_assertions] Define dataspace status assertions for queued, fetching, running, complete, failed, cancelled, and result-ready.
- [x] [parallel] r[molten.blob_ref_jobs.receipts] Emit receipts for submission, fetch, verification, admission, execution, result, cleanup, and denial.

## Phase 2: Worker flow

- [x] [serial] r[molten.blob_ref_jobs.local_worker] Implement a local deterministic worker that fetches refs from local chunk/blob store, verifies, runs, and stores outputs by ref.
- [x] [serial] r[molten.blob_ref_jobs.content_verification] Verify executable/input chunk manifests or blob hashes before execution.
- [x] [parallel] r[molten.blob_ref_jobs.provenance_policy] Gate executable refs by artifact provenance and effect/admission policy.
- [x] [parallel] r[molten.blob_ref_jobs.retention_pins] Pin executable/input/output refs while active and release according to retention policy.

## Phase 3: Integration and tests

- [x] [serial] r[molten.blob_ref_jobs.job_dag_integration] Use blob/chunk refs as executable and partition inputs for distributed job DAG stages.
- [x] [parallel] r[molten.blob_ref_jobs.replay_integration] Include job refs, fetch receipts, and handler profile in deterministic replay identity.
- [x] [serial] r[molten.blob_ref_jobs.local_tests] Add tests for submitting, fetching, verifying, running, and completing a local ref-backed job.
- [x] [parallel] r[molten.blob_ref_jobs.property_tests] Add Hegel property tests for no-inline-large-bytes, content verification before execution, and pin lifecycle invariants.
