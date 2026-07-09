## Tasks

- [ ] [serial] r[molten.upgrades.structured_session_artifacts] Define structured upgrade session artifacts for alias moves, artifact replacements, schema migrations, protocol drains, policy updates, handler-profile changes, transcript rewrites, and cleanup.
- [ ] [serial] r[molten.upgrades.receipt_backed_task_state] Store mutable task progress as receipt-backed metadata that points to the immutable plan artifact instead of changing plan identity.
- [ ] [parallel] r[molten.upgrades.cutover_gate_binding] Require impact query, compatibility, migration, protocol-session, replay, policy, capability, and retention evidence before cutover or cleanup side effects.
- [ ] [parallel] r[molten.upgrades.no_source_control_replacement] Document that upgrade sessions do not replace Git, Cargo, Nix, Cairn changes, or human review and do not adopt UCM behavior.
- [ ] [serial] r[molten.upgrades.session_validation] Add positive and negative fixtures for gated cutover, stale impact evidence, missing protocol drain, incomplete migration, failed replay, unauthorized alias update, and destructive cleanup denial.