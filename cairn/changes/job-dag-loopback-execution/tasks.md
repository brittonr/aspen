## Phase 1: Canonical execution records

- [x] [serial] r[molten.job_dag_loopback_execution.request_dto] Define `job-execution-request-v1` with job ref, admission receipt ref, target peer, selected stages, storage/cache/chunk profile refs, policy refs, capability refs, resource refs, and admission-required checks.
- [x] [serial] r[molten.job_dag_loopback_execution.receipt_dto] Define `job-execution-receipt-v1` with operation, decision, job ref, request ref, admission ref, sync ref, target peer, closure refs, authority refs, stage receipt refs, output refs, diagnostics, refs, and checks.
- [x] [parallel] r[molten.job_dag_loopback_execution.ledger_classification] Classify execution requests and receipts in ledger/catalog surfaces.

## Phase 2: Admission receipt verifier

- [x] [serial] r[molten.job_dag_loopback_execution.admission_required] Require a readable `job-admission-receipt-v1` before target execution.
- [x] [serial] r[molten.job_dag_loopback_execution.admission_pass] Deny execution unless the admission receipt decision is `pass`.
- [x] [serial] r[molten.job_dag_loopback_execution.binding_checks] Verify admission job ref, target peer, selected stages, sync ref, closure refs, authority refs, and resource refs match the execution request.
- [x] [parallel] r[molten.job_dag_loopback_execution.admission_checkset] Require admission checks for target closure, Trellis topology, executable artifact gate, capability authority context, resource profile, and no-execution.
- [x] [parallel] r[molten.job_dag_loopback_execution.deny_receipts] Emit canonical deny receipts for missing, denied, stale, mismatched, or tampered admission evidence without running stages.

## Phase 3: Target closure revalidation

- [x] [serial] r[molten.job_dag_loopback_execution.recompute_closure] Recompute target job/stage closure from the target registry immediately before execution.
- [x] [serial] r[molten.job_dag_loopback_execution.closure_match] Compare recomputed closure with admitted closure refs and deny divergence.
- [x] [parallel] r[molten.job_dag_loopback_execution.no_source_registry] Ensure loopback execution accepts no source-registry argument and reads only target registry/storage/cache/chunks.
- [x] [parallel] r[molten.job_dag_loopback_execution.no_mobile_closures] Preserve executable artifact/no-mobile-closure boundaries from admission through execution.

## Phase 4: Loopback execution

- [x] [serial] r[molten.job_dag_loopback_execution.execute_target] Run the existing deterministic job executor against the target registry and target storage/cache/chunk roots only after admission verification passes.
- [x] [serial] r[molten.job_dag_loopback_execution.receipt_binding] Bind admission receipt ref, sync ref, authority admission refs, stage receipts, output refs, target peer, and resource refs in the execution receipt.
- [x] [parallel] r[molten.job_dag_loopback_execution.output_equivalence] Assert loopback target outputs match equivalent local execution for supported deterministic stages.

## Phase 5: CLI and tests

- [x] [serial] r[molten.job_dag_loopback_execution.cli] Add `molten test job execute-loopback` with target registry/storage/cache/chunks/admission receipt arguments and no source-registry argument.
- [x] [parallel] r[molten.job_dag_loopback_execution.tests_pass] Test successful sync + capability-backed admission + execute-loopback.
- [x] [parallel] r[molten.job_dag_loopback_execution.tests_missing_admission] Test denial without an admission receipt.
- [x] [parallel] r[molten.job_dag_loopback_execution.tests_deny_admission] Test denial with a deny admission receipt.
- [x] [parallel] r[molten.job_dag_loopback_execution.tests_mismatch] Test denial for job mismatch, target peer mismatch, and stale closure divergence.
- [x] [parallel] r[molten.job_dag_loopback_execution.tests_receipt_refs] Test execution receipts bind admission, sync, authority, resource, stage, and output refs.
