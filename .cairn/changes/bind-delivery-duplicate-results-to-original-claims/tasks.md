# F08 implementation tasks

All tasks require separate implementation authorization. No task records completed work.

- [ ] [serial] Run the smallest existing coordination-delivery duplicate and completion baselines before core edits. Record failures. r[molten.audit_f08.validation_claims]
- [ ] [serial] Move `audit_duplicate_claim_does_not_return_another_consumers_token` into normal repository tests with named fixture constants. r[molten.audit_f08.original_binding]
- [ ] [serial] Add positive immediate replay and negative expiry/reclaim, completed-item, and non-claim duplicate cases. r[molten.audit_f08.original_binding] r[molten.audit_f08.unavailable_result]
- [ ] [serial] Resolve caller handling of absent original tokens and canonical response compatibility before implementation. r[molten.audit_f08.unavailable_result]
- [ ] [serial] Implement pure-core token-reference matching without current-item substitution or unbounded historical token storage. r[molten.audit_f08.original_binding] r[molten.audit_f08.unavailable_result]
- [ ] [serial] Preserve imperative-shell no-commit replay and separate completion-owner, delegated-authority, deadline, and fencing checks. r[molten.audit_f08.state_authority]
- [ ] [serial] Add service tests that assert unchanged durable revision, queue state, timers, and worker effects after duplicate or denied requests. r[molten.audit_f08.state_authority]
- [ ] [serial] Add positive delegated-completion and negative wrong-owner, stale-token, and conflicting-request controls. r[molten.audit_f08.state_authority]
- [ ] [serial] Add Redb reopen and canonical adapter round-trip tests, including absent tokens, malformed records, and original-operation binding. r[molten.audit_f08.original_binding] r[molten.audit_f08.validation_claims]
- [ ] [serial] Add replay compatibility tests that preserve historical records and reject later-token result substitution. r[molten.audit_f08.original_binding] r[molten.audit_f08.unavailable_result]
- [ ] [serial] Update delivery docs with unavailable-token semantics, the maintenance owner, and wrong-response versus authority-bypass limits. r[molten.audit_f08.validation_claims]
- [ ] [serial] Rerun focused baselines and run focused Octet error gates and Clippy with `-D warnings`. r[molten.audit_f08.validation_claims]
- [ ] [serial] Run workspace tests across all targets and compatible features, relevant Nix delivery checks, and required repository gates. r[molten.audit_f08.validation_claims]
- [ ] [serial] Run required Cairn validation, traceability, compatibility, and evidence gates without weaker checks. Record exact results and blockers. r[molten.audit_f08.validation_claims]
