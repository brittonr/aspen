# Tasks: peer-session-lifecycle

## Phase 1: Canonical peer/session model

- [x] [serial] r[molten.peer_session.lifecycle_model] Define canonical `peer-profile-v1` and `peer-session-v1` records with identity, endpoint, scope, resource, freshness, revocation, and evidence bindings.
- [x] [serial] r[molten.peer_session.lifecycle_transitions] Implement the pure peer lifecycle reducer and transition receipts for discovered, invited, handshaking, negotiated, admitted, connected, expired, revoked, and quarantined states.
- [x] [parallel] r[molten.peer_session.authority_boundary] Preserve explicit denial when transport observations, profile matches, or connected sessions are presented as authority.

## Phase 2: Node state and configuration

- [x] [serial] r[molten.peer_session.node_state_table] Add the node-local peer read model and bounded indexes for session lookup, status, and diagnostics.
- [x] [parallel] r[molten.peer_session.nickel_config] Add typed Nickel contracts and positive/negative fixtures for static peer profiles and exported peer config.
- [x] [parallel] r[molten.peer_session.live_ticket_session_binding] Bind existing live tickets, peer admissions, and authority imports into peer-session state without changing their canonical receipt semantics.

## Phase 3: CLI and diagnostics

- [x] [serial] r[molten.peer_session.peer_cli] Add `molten peer invite create`, `invite accept`, `connect`, `status`, `revoke`, and `diagnose` shells over the peer-session core.
- [x] [serial] r[molten.peer_session.diagnostics] Report transport reachability, bootstrap admission, capability admission, authority grant, resource/policy admission, replay/idempotency, and next missing step in diagnostics.
- [x] [parallel] r[molten.peer_session.positive_negative_tests] Add positive session lifecycle tests and negative tests for stale ticket, wrong topic, missing admission, missing authority, revoked profile, unsafe config, and transport-only evidence.

## Phase 4: Validation

- [x] [serial] r[molten.peer_session.validation] Run focused peer/session tests, Nickel fixture validation, `cargo fmt --check`, peer-related cargo tests, and Cairn validation before claiming the lifecycle slice complete.
