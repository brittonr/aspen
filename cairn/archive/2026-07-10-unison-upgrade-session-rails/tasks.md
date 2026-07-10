## Tasks

- [x] [serial] r[molten.upgrades.structured_session_artifacts] Define structured upgrade session artifacts for alias moves, artifact replacements, schema migrations, protocol drains, policy updates, handler-profile changes, transcript rewrites, and cleanup.
  - Extended supported upgrade task kinds and plan checks for artifact replacement, schema/storage migration, protocol drain, policy/handler-profile update, transcript replay, and cleanup surfaces.
- [x] [serial] r[molten.upgrades.receipt_backed_task_state] Store mutable task progress as receipt-backed metadata that points to the immutable plan artifact instead of changing plan identity.
  - Status reads now accept only stored passing receipts that bind the same immutable plan ref and task id; bogus checkbox/status metadata remains incomplete.
- [x] [parallel] r[molten.upgrades.cutover_gate_binding] Require impact query, compatibility, migration, protocol-session, replay, policy, capability, and retention evidence before cutover or cleanup side effects.
  - Added cutover readiness checks for exact refs, impact, compatibility, replay, migration, protocol drain, policy, capability, source-gate/review, and rollback bindings; cleanup tasks require retention and dependency-impact evidence.
- [x] [parallel] r[molten.upgrades.no_source_control_replacement] Document that upgrade sessions do not replace Git, Cargo, Nix, Cairn changes, or human review and do not adopt UCM behavior.
  - Added plan checks and validation denial for UCM compatibility or source-control/build/review replacement claims.
- [x] [serial] r[molten.upgrades.session_validation] Add positive and negative fixtures for gated cutover, stale impact evidence, missing protocol drain, incomplete migration, failed replay, unauthorized alias update, and destructive cleanup denial.
  - Added/extended positive and negative upgrade-session fixtures. Validation evidence: `nix develop -c cargo test --lib migrations::tests -- --nocapture` passes.