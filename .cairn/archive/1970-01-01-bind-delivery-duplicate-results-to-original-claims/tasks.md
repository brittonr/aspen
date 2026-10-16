# F08 implementation tasks

Implementation is authorized under the bounded completion campaign. Checked tasks
record implemented and exercised work. Current acceptance and bounded review are
recorded in [evidence/2026-10-16-acceptance.json](evidence/2026-10-16-acceptance.json).

- [x] [serial] Run the smallest existing coordination-delivery duplicate and completion baselines before core edits. Record failures. r[molten.audit_f08.validation_claims]
- [x] [serial] Move `audit_duplicate_claim_does_not_return_another_consumers_token` into normal repository tests with named fixture constants. r[molten.audit_f08.original_binding]
- [x] [serial] Add positive immediate replay and negative expiry/reclaim, completed-item, and non-claim duplicate cases. r[molten.audit_f08.original_binding] r[molten.audit_f08.unavailable_result]
- [x] [serial] Resolve caller handling of absent original tokens and canonical response compatibility before implementation. r[molten.audit_f08.unavailable_result]
- [x] [serial] Implement pure-core token-reference matching without current-item substitution or unbounded historical token storage. r[molten.audit_f08.original_binding] r[molten.audit_f08.unavailable_result]
- [x] [serial] Preserve imperative-shell no-commit replay and separate completion-owner, delegated-authority, deadline, and fencing checks. r[molten.audit_f08.state_authority]
- [x] [serial] Add service tests that assert unchanged durable revision, queue state, timers, and worker effects after duplicate or denied requests. r[molten.audit_f08.state_authority]
- [x] [serial] Add positive delegated-completion and negative wrong-owner, stale-token, and conflicting-request controls. r[molten.audit_f08.state_authority]
- [x] [serial] Add Redb reopen and canonical adapter round-trip tests, including absent tokens, malformed records, and original-operation binding. r[molten.audit_f08.original_binding] r[molten.audit_f08.validation_claims]
- [x] [serial] Add replay compatibility tests that preserve historical records and reject later-token result substitution. r[molten.audit_f08.original_binding] r[molten.audit_f08.unavailable_result]
- [x] [serial] Update delivery docs with unavailable-token semantics, the maintenance owner, and wrong-response versus authority-bypass limits. r[molten.audit_f08.validation_claims]
- [x] [serial] Rerun focused baselines and run focused Octet error gates and Clippy with `-D warnings`. r[molten.audit_f08.validation_claims]
- [x] [serial] Run workspace tests across all targets and compatible features, relevant Nix delivery checks, and required repository gates. r[molten.audit_f08.validation_claims]
- [x] [serial] Run required Cairn validation, traceability, compatibility, and evidence gates without weaker checks. Record exact results and blockers. r[molten.audit_f08.validation_claims]

## Earlier 2026-10-15 acceptance attempt

Fresh execution and the mandatory strict-gate blocker are recorded in
[`evidence/2026-10-15-verification.md`](evidence/2026-10-15-verification.md).
Default-feature workspace tests, focused delivery Nix checks, formatting, and
Clippy passed. Required repository-wide strict Octet acceptance did not pass;
the remaining gate tasks are not complete and this change is not archive-ready.

## 2026-10-16 acceptance

Attempt 15/20 binds the unchanged product source to the recorded passing
attempt-13 workspace, compatible-feature, strict-source, and Nix acceptance.
The new external public consumer exercises the real capability-rooted Redb
adapter, durable reopen, original-result reconstruction, and independent
completion admission. Its 26 outcomes include explicit rejection/no-mutation
controls; timer and status ports remain controlled observations.

Native verification-run receipts bind actual exit-zero and expected-deny
exit-one executions. The existing receipt-backed traceability builder covers
all four F08 requirements with no missing positive/negative evidence, stale
targets, compatibility tuples, or exemptions. This is change-scoped evidence,
not whole-repository release coverage. Historical failed evidence above remains
unchanged; native sync/archive still require their own matching receipts.

