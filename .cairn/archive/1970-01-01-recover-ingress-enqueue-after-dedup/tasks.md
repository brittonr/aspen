## Implementation tasks

Implementation follows the operator's continuation request. Checked items record completed work; verification evidence states execution boundaries and retained failures.

- [x] [serial] Run `control_ingress_enqueues_once_and_preserves_provenance_gate` and `control_ingress_denies_missing_authority_before_enqueue` before core edits. Record baseline results. r[molten.audit_f03.validation]
- [x] [serial] Add the F03 dedup-before-enqueue reproduction to normal repository tests with a successful fresh-enqueue control. r[molten.audit_f03.recover_publication] r[molten.audit_f03.validation]
- [x] [serial] Agree on uncertainty and quarantine inputs with `recover-from-storage-faults` without adding consensus recovery scope. r[molten.audit_f03.uncertainty]
- [x] [serial] Define pure reconciliation decisions from exact dedup, intent, inbox, dispatch, and storage-health observations. r[molten.audit_f03.recover_publication] r[molten.audit_f03.no_blind_replay]
- [x] [serial] Select the smallest recoverable publication mechanism after review of existing component contracts. r[molten.audit_f03.recover_publication]
- [x] [serial] Implement shell publication and reopen ordering through application-owned capabilities without core filesystem or Redb access. r[molten.audit_f03.recover_publication] r[molten.audit_f03.uncertainty]
- [x] [serial] Add adapter tests for faults before dedup, after dedup, after enqueue, after dispatch, and before receipt publication. r[molten.audit_f03.no_blind_replay] r[molten.audit_f03.validation]
- [x] [serial] Add negative tests for ambiguous commits, unavailable observations, quarantined storage, and conflicting payloads. r[molten.audit_f03.uncertainty] r[molten.audit_f03.preserve_state]
- [x] [serial] Assert rejection preserves dedup, inbox, and dispatch state without blind enqueue or automatic control-operation replay. r[molten.audit_f03.preserve_state] r[molten.audit_f03.no_blind_replay]
- [x] [serial] Review legacy dedup compatibility and receipt versions for observed publication, incomplete recovery, and unknown outcomes. r[molten.audit_f03.recover_publication] r[molten.audit_f03.uncertainty]
- [x] [serial] Document local recovery, replay limits, owner boundaries, and the absence of exactly-once or consensus guarantees. r[molten.audit_f03.no_blind_replay] r[molten.audit_f03.validation]
- [x] [serial] Repeat focused baseline and reopen regressions after edits. Distinguish simulated faults from persistence observations. r[molten.audit_f03.validation]
- [x] [serial] Run focused Octet error checks and Clippy with `-D warnings`, without weaker checks. r[molten.audit_f03.validation]
- [x] [serial] Run workspace tests across all targets and compatible features, relevant Nix delivery checks, and required flake gates. r[molten.audit_f03.validation]
- [x] [serial] Run required Cairn validation, traceability, and package gates before completion review. Record unresolved evidence explicitly. r[molten.audit_f03.validation]

Executed observations and their limits are recorded in the [F03 verification record](evidence/2026-10-20-verification.md). The immutable baseline replay is retrospective pre-fix evidence; it does not rewrite the historical audit or substitute synthetic startup evidence for F02.
