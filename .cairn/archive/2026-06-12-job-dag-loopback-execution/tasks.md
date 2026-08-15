## Phase 1: Canonical execution records

- [x] [serial] r[molten.job_dag_loopback_execution.spec.receipts] Define `job-execution-request-v1` with job ref, admission receipt ref, target peer, selected stages, storage/cache/chunk profile refs, policy refs, capability refs, resource refs, and admission-required checks.
- [x] [serial] r[molten.job_dag_loopback_execution.spec.receipts] Define `job-execution-receipt-v1` with operation, decision, job ref, request ref, admission ref, sync ref, target peer, closure refs, authority refs, stage receipt refs, output refs, diagnostics, refs, and checks.
- [x] [parallel] r[molten.job_dag_loopback_execution.spec.receipts] Classify execution requests and receipts in ledger/catalog surfaces.

## Phase 2: Admission receipt verifier

- [x] [serial] r[molten.job_dag_loopback_execution.spec.admission_required] Require a readable `job-admission-receipt-v1` before target execution.
- [x] [serial] r[molten.job_dag_loopback_execution.spec.admission_required] Deny execution unless the admission receipt decision is `pass`.
- [x] [serial] r[molten.job_dag_loopback_execution.spec.admission_binding] Verify admission job ref, target peer, selected stages, sync ref, closure refs, authority refs, and resource refs match the execution request.
- [x] [parallel] r[molten.job_dag_loopback_execution.spec.admission_binding] Require admission checks for target closure, Trellis topology, executable artifact gate, capability authority context, resource profile, and no-execution.
- [x] [parallel] r[molten.job_dag_loopback_execution.spec.receipts] Emit canonical deny receipts for missing, denied, stale, mismatched, or tampered admission evidence without running stages.

## Phase 3: Target closure revalidation

- [x] [serial] r[molten.job_dag_loopback_execution.spec.closure_revalidate] Recompute target job/stage closure from the target registry immediately before execution.
- [x] [serial] r[molten.job_dag_loopback_execution.spec.closure_revalidate] Compare recomputed closure with admitted closure refs and deny divergence.
- [x] [parallel] r[molten.job_dag_loopback_execution.spec.target_only] Ensure loopback execution accepts no source-registry argument and reads only target registry/storage/cache/chunks.
- [x] [parallel] r[molten.job_dag_loopback_execution.spec.target_only] Preserve executable artifact/no-mobile-closure boundaries from admission through execution.

## Phase 4: Loopback execution

- [x] [serial] r[molten.job_dag_loopback_execution.spec.target_only] Run the existing deterministic job executor against the target registry and target storage/cache/chunk roots only after admission verification passes.
- [x] [serial] r[molten.job_dag_loopback_execution.spec.receipts] Bind admission receipt ref, sync ref, authority admission refs, stage receipts, output refs, target peer, and resource refs in the execution receipt.
- [x] [parallel] r[molten.job_dag_loopback_execution.spec.receipts] Assert loopback target outputs match equivalent local execution for supported deterministic stages.

## Phase 5: CLI and tests

- [x] [serial] r[molten.job_dag_loopback_execution.spec.receipts] Add `molten test job execute-loopback` with target registry/storage/cache/chunks/admission receipt arguments and no source-registry argument.
- [x] [parallel] r[molten.job_dag_loopback_execution.spec.receipts] Test successful sync + capability-backed admission + execute-loopback.
- [x] [parallel] r[molten.job_dag_loopback_execution.spec.admission_required] Test denial without an admission receipt.
- [x] [parallel] r[molten.job_dag_loopback_execution.spec.admission_required] Test denial with a deny admission receipt.
- [x] [parallel] r[molten.job_dag_loopback_execution.spec.closure_revalidate] Test denial for job mismatch, target peer mismatch, and stale closure divergence.
- [x] [parallel] r[molten.job_dag_loopback_execution.spec.receipts] Test execution receipts bind admission, sync, authority, resource, stage, and output refs.
