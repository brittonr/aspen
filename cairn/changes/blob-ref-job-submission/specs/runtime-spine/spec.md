# Runtime Spine Delta: Blob-ref Job Submission

### Requirement: Blob-ref job submissions
r[molten.blob_ref_jobs.payload_model] Molten MUST expose canonical job submission artifacts that reference executable and input content by artifact, blob, or chunk refs instead of embedding large bytes.
r[molten.blob_ref_jobs.no_inline_large_bytes] Molten MUST reject ref-backed job submissions that embed large executable, input, output, log, or dataset bytes instead of content refs.

#### Scenario: Content-ref-only submission
- GIVEN a job id, operation id, executable ref, input refs, size/format hints, authority context ref, policy refs, provenance refs, effect refs, and output mode
- WHEN an operator creates a ref-backed job submission
- THEN Molten MUST emit a canonical `job-ref-submission-v1` artifact that records those refs and the checks `content-refs-only` and `no-inline-large-bytes`

#### Scenario: Inline large content is denied
- GIVEN a ref-backed job submission that contains inline executable, input, dataset, output, or log bytes
- WHEN Molten parses or admits the submission
- THEN Molten MUST deny it before execution and require content refs for large content

### Requirement: Blob-ref worker fetch and verification
r[molten.blob_ref_jobs.local_worker] Molten MUST provide a deterministic local worker that fetches refs from a local chunk/blob store, verifies them, runs the declared handler profile, and stores outputs by ref.
r[molten.blob_ref_jobs.content_verification] Molten MUST verify executable/input chunk manifests or blob hashes before execution.
r[molten.blob_ref_jobs.provenance_policy] Molten MUST require explicit provenance and effect/policy refs for executable refs before treating the run as admitted execution evidence.
r[molten.blob_ref_jobs.retention_pins] Molten MUST pin executable, input, and output refs while a job is active and emit cleanup evidence when active pins are released.

#### Scenario: Verified local worker execution
- GIVEN a valid ref-backed job submission and a local chunk store containing the executable and input manifests
- WHEN the deterministic local worker executes the job
- THEN Molten MUST read the manifests, verify them before execution, pin active content refs, run the declared handler profile, store result bytes as content refs, and emit status and receipt artifacts for the run

#### Scenario: Missing or tampered content ref
- GIVEN a ref-backed job submission whose executable or input ref cannot be fetched or verified
- WHEN the deterministic local worker attempts execution
- THEN Molten MUST deny the run before invoking the handler and emit diagnostics plus a canonical denial receipt

### Requirement: Blob-ref job status and receipt evidence
r[molten.blob_ref_jobs.status_assertions] Molten MUST expose job status assertions for queued, fetching, running, complete, failed, cancelled, and result-ready states.
r[molten.blob_ref_jobs.receipts] Molten MUST emit receipts that bind submission, fetch, verification, admission, execution, result, cleanup, and denial evidence.
r[molten.blob_ref_jobs.replay_integration] Molten MUST include job refs, fetch receipts, verification receipts, and handler profile identity in deterministic replay identity.

#### Scenario: Status lifecycle evidence
- GIVEN a ref-backed job execution
- WHEN the worker progresses through queued, fetching, running, result-ready, complete, failed, or cancelled states
- THEN Molten MUST emit canonical `job-ref-status-v1` evidence records that bind the submission ref, operation id, output refs, and checks for the state

#### Scenario: Receipt replay identity
- GIVEN a ref-backed job execution receipt
- WHEN Molten summarizes, stores, or replays the receipt
- THEN the receipt MUST include the submission ref, job id, operation id, executable ref, input refs, status refs, fetch refs, verification refs, pin refs, cleanup refs, output ref, handler profile outcome, diagnostics, and pass/fail checks needed to reproduce the decision

### Requirement: Blob-ref job DAG integration
r[molten.blob_ref_jobs.job_dag_integration] Molten SHOULD integrate ref-backed job submissions with existing job DAG and delivery evidence surfaces without granting implicit authority.
r[molten.blob_ref_jobs.local_tests] Molten MUST test submitting, fetching, verifying, running, and completing a local ref-backed job.
r[molten.blob_ref_jobs.property_tests] Molten MUST include property coverage for no-inline-large-bytes, content verification before execution, and pin lifecycle invariants.

#### Scenario: CLI and ledger integration
- GIVEN a ref-backed job submission and execution receipt
- WHEN an operator uses the job CLI status or receipt commands
- THEN Molten SHOULD display the ref-backed receipt alongside existing job DAG receipts and ledger classifications

#### Scenario: Evidence only
- GIVEN a passing ref-backed job receipt
- WHEN another runtime operation evaluates authority, provenance, policy, resource, or transport admission
- THEN the ref-backed receipt MUST be treated as execution evidence only and MUST NOT grant authority, provenance, policy, resource, transport, or source-gate trust by itself
