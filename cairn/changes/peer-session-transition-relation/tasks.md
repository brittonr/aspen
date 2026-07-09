# Tasks: peer-session-transition-relation

- [ ] [serial] r[molten.peer_session.transition_relation_closed] Define the reviewed peer-session states, events, allowed transition table, and guard classes in the pure peer-session core.
- [ ] [serial] r[molten.peer_session.transition_relation_closed] Refactor peer transition evaluation so every pass requires a table entry and every missing entry emits a deny decision without advancing state.
- [ ] [parallel] r[molten.peer_session.terminal_quarantine_guards] Add explicit terminal and quarantine guards for expired, revoked, and quarantined sessions, including recovery evidence requirements.
- [ ] [parallel] r[molten.peer_session.transition_receipt_binding] Extend or add transition receipts that bind before-state, event, target, after-state or preserved-state refs, guard evidence, decision, diagnostics, and evidence-only authority caveats.
- [ ] [parallel] r[molten.peer_session.transition_trace_tests] Add positive fixtures for admitted peer progression and negative fixtures for invalid skips, wrong topic, stale tickets, revoked evidence, quarantine bypass, and transport-only evidence.
- [ ] [serial] r[molten.peer_session.transition_trace_tests] Run focused peer-session tests and Cairn validation, then record validation evidence in implementation notes.