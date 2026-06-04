## Phase 1: Authority context resolution

- [x] [serial] r[molten.job_dag_capability_admission.context_lookup] Resolve admission capability refs to `authority-context-v1` payload refs in the target artifact registry.
- [x] [serial] r[molten.job_dag_capability_admission.authority_admit] Reuse `admit_authority` for `job:execute` scoped to the job ref.
- [x] [parallel] r[molten.job_dag_capability_admission.deny_placeholders] Deny placeholder, missing, wrong-scope, wrong-capability, expired, revoked, or attenuated authority contexts.

## Phase 2: Evidence binding

- [x] [serial] r[molten.job_dag_capability_admission.plan_receipts] Add authority admission receipt refs to job admission plans and receipts.
- [x] [parallel] r[molten.job_dag_capability_admission.checks] Add `capability-authority-context` pass/fail checks to admission plans and receipts.
- [x] [parallel] r[molten.job_dag_capability_admission.no_artifact_authority] Preserve the invariant that synced artifact possession alone grants no execution authority.

## Phase 3: Tests

- [x] [parallel] r[molten.job_dag_capability_admission.tests_pass] Test pass admission with a target authority context granting `job:execute` for the job ref.
- [x] [parallel] r[molten.job_dag_capability_admission.tests_deny] Test denial for missing/placeholder capability refs.
- [x] [parallel] r[molten.job_dag_capability_admission.no_execution] Assert capability admission remains non-executing.
