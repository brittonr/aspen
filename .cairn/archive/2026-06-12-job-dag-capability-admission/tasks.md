## Phase 1: Authority context resolution

- [x] [serial] r[molten.job_dag_capability_admission.spec.context_resolution] Resolve admission capability refs to `authority-context-v1` payload refs in the target artifact registry.
- [x] [serial] r[molten.job_dag_capability_admission.spec.job_execute_scope] Reuse `admit_authority` for `job:execute` scoped to the job ref.
- [x] [parallel] r[molten.job_dag_capability_admission.spec.context_resolution] Deny placeholder, missing, wrong-scope, wrong-capability, expired, revoked, or attenuated authority contexts.

## Phase 2: Evidence binding

- [x] [serial] r[molten.job_dag_capability_admission.spec.job_execute_scope] Add authority admission receipt refs to job admission plans and receipts.
- [x] [parallel] r[molten.job_dag_capability_admission.spec.job_execute_scope] Add `capability-authority-context` pass/fail checks to admission plans and receipts.
- [x] [parallel] r[molten.job_dag_capability_admission.spec.no_artifact_authority] Preserve the invariant that synced artifact possession alone grants no execution authority.

## Phase 3: Tests

- [x] [parallel] r[molten.job_dag_capability_admission.spec.job_execute_scope] Test pass admission with a target authority context granting `job:execute` for the job ref.
- [x] [parallel] r[molten.job_dag_capability_admission.spec.context_resolution] Test denial for missing/placeholder capability refs.
- [x] [parallel] r[molten.job_dag_capability_admission.spec.no_artifact_authority] Assert capability admission remains non-executing.
