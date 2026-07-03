# Tasks: authority-peer-admission-state-proof

## Phase 1: Authority state proof

- [ ] [serial] r[molten.authority_peer_state_proof.current_scoped_grant] Define or expose pure authority grant currentness checks over scope, epoch, attenuation, expiry, key, and revocation state.
- [ ] [parallel] r[molten.authority_peer_state_proof.import_not_authority] Add diagnostics that imported grants and tickets remain candidates until normal admission passes.
- [ ] [parallel] r[molten.authority_peer_state_proof.replay_no_current_authority] Add replay-vs-current authority distinction checks.

## Phase 2: Peer admission proof

- [ ] [serial] r[molten.peer_admission_state_proof.ticket_scope] Add pure live ticket/peer admission scope checks for node, peer, topic, endpoint, freshness, and policy evidence.
- [ ] [parallel] r[molten.peer_admission_state_proof.transport_not_bootstrap] Add explicit rejection for neighbor/listener/send receipts used as bootstrap or authority.

## Phase 3: Tests and validation

- [ ] [parallel] r[molten.authority_peer_state_proof.current_scoped_grant] Add positive scoped admission and negative revoked, expired, wrong-scope, stale-epoch, and wrong-key tests.
- [ ] [parallel] r[molten.peer_admission_state_proof.ticket_scope] r[molten.peer_admission_state_proof.transport_not_bootstrap] Add ticket mismatch and transport-only denial tests.
- [ ] [serial] r[molten.authority_peer_state_proof.current_scoped_grant] r[molten.authority_peer_state_proof.import_not_authority] r[molten.authority_peer_state_proof.replay_no_current_authority] r[molten.peer_admission_state_proof.ticket_scope] r[molten.peer_admission_state_proof.transport_not_bootstrap] Bind proof trace refs and run `cargo test authority peer node`.
