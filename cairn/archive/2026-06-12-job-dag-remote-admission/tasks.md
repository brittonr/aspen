## Phase 1: Canonical admission records

- [x] [serial] r[molten.job_dag_remote_admission.spec.request] Define `job-admission-request-v1` with job ref, sync evidence ref, selected stages, target peer, policy refs, capability refs, evidence refs, resource refs, and no-execution checks.
- [x] [serial] r[molten.job_dag_remote_admission.spec.receipts] Define `job-admission-plan-v1` with request ref, job ref, sync ref, closure refs, Trellis topology order, stage verdicts, resource verdict, decision, and checks.
- [x] [serial] r[molten.job_dag_remote_admission.spec.receipts] Define `job-admission-receipt-v1` for advisory plan and loopback admission operations.
- [x] [parallel] r[molten.job_dag_remote_admission.spec.receipts] Classify admission requests, plans, and receipts in ledger/catalog surfaces.

## Phase 2: Target closure and topology verifier

- [x] [serial] r[molten.job_dag_remote_admission.spec.target_closure] Resolve the target job artifact by canonical artifact ref and verify the parsed job DAG ref.
- [x] [serial] r[molten.job_dag_remote_admission.spec.target_closure] Recompute selected job/stage closure roots from the target registry and deny missing or tampered artifacts.
- [x] [serial] r[molten.job_dag_remote_admission.spec.target_closure] Bind admission to referenced sync plan/receipt evidence and deny closure divergence.
- [x] [parallel] r[molten.job_dag_remote_admission.spec.trellis_topology] Use Trellis topology/job-DAG primitives for canonical stage order, cycle rejection, unknown-stage rejection, and selected-dependency satisfaction.
- [x] [parallel] r[molten.job_dag_remote_admission.spec.target_closure] Ensure admission never uses mutable names, paths, mtimes, URLs, or display metadata for artifact identity.

## Phase 3: Executable, authority, and resource gates

- [x] [serial] r[molten.job_dag_remote_admission.spec.executable_artifact_gate] Deny executable stages without admitted artifact-backed stage operations.
- [x] [parallel] r[molten.job_dag_remote_admission.spec.executable_artifact_gate] Deny raw closures, inline scripts, shell commands, host paths, mutable image tags, unverified remote URLs, and mobile code configs.
- [x] [parallel] r[molten.job_dag_remote_admission.spec.authority_resource] Require explicit non-empty policy, capability, evidence, and resource refs for pass admission.
- [x] [parallel] r[molten.job_dag_remote_admission.spec.authority_resource] Bind job profile/resource governance refs and deny over-budget stage selections before execution.
- [x] [parallel] r[molten.job_dag_remote_admission.spec.request] Bind target peer identity without granting authority from sync success alone.

## Phase 4: CLI and tests

- [x] [serial] r[molten.job_dag_remote_admission.spec.receipts] Add `molten test job admit-plan` and `molten test job admit-loopback` commands.
- [x] [parallel] r[molten.job_dag_remote_admission.spec.receipts] Add loopback tests showing admission passes after successful sync with explicit authority/resource refs.
- [x] [parallel] r[molten.job_dag_remote_admission.spec.target_closure] Add denial tests for missing dependencies, tampered artifacts, and sync-closure divergence.
- [x] [parallel] r[molten.job_dag_remote_admission.spec.executable_artifact_gate] Add denial tests for non-artifact executable configs, raw/mobile closures, paths, shell commands, and URLs.
- [x] [parallel] r[molten.job_dag_remote_admission.spec.authority_resource] Add denial tests for absent policy/capability/evidence/resource refs and over-budget resources.
- [x] [parallel] r[molten.job_dag_remote_admission.spec.receipts] Assert admission emits no-execution checks and never runs stage operations.
