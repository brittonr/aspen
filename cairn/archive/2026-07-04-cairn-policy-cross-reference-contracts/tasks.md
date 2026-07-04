# Tasks: cairn-policy-cross-reference-contracts

- [x] [serial] r[molten.project.cairn_policy_integrity.references] Add pure whole-policy Nickel helpers that index artifact ids, replay case ids, replay group ids, receipt schema commands, and receipt contract bindings.
- [x] [serial] r[molten.project.cairn_policy_integrity.references] Reject unknown artifact dependencies, stale determinism replay references, and receipt contract/schema command mismatches at policy export time.
- [x] [serial] r[molten.project.cairn_policy_integrity.uniqueness] Reject duplicate artifact ids, marker ids, marker tokens, replay ids, and ambiguous receipt schema command entries.
- [x] [parallel] r[molten.project.cairn_policy_integrity.references] Add valid and invalid Cairn policy fixtures for cross-reference resolution failures.
- [x] [parallel] r[molten.project.cairn_policy_integrity.uniqueness] Add negative fixtures for duplicate marker, artifact, replay, and receipt-schema identities.
- [x] [serial] r[molten.project.cairn_policy_integrity.references] Run policy export/check validation and `cairn validate --root .`, or record the blocker and next best check.
