# Tasks: capability-token-spine

## Phase 1: Token and proofset model

- [x] [serial] r[molten.capability_token.record_model] Define canonical `capability-token-v1`, `capability-proofset-v1`, and `capability-admission-receipt-v1` records with issuer, holder, session/context, resource, ability, scope, attenuation, caveats, expiry, revocation, policy, resource, delegation, and evidence bindings.
- [x] [serial] r[molten.capability_token.taxonomy] Document and encode the taxonomy separating identity refs, transport receipts, peer sessions, handoff bundles, bootstrap tickets, read tokens, write tokens, promotion tokens, authority tokens, and membership evidence.
- [x] [parallel] r[molten.capability_token.basalt_ucan_seam] Preserve a Basalt/UCAN proof replacement seam while supporting local deterministic capability fixtures for tests.

## Phase 2: Admission core

- [x] [serial] r[molten.capability_token.admission_law] Implement pure capability proofset admission over exact holder/session/context, resource, ability, scope, attenuation, caveats, expiry, revocation, key-currentness, policy refs, and resource refs.
- [x] [serial] r[molten.capability_token.import_not_authority] Enforce that imported tokens, handoff-carried tokens, receipts, and proofsets remain evidence candidates until the admission law passes for the requested action.
- [x] [parallel] r[molten.capability_token.diagnostics] Add diagnostics that name missing/wrong holder, session, ability, scope, caveat, revocation, expiry, policy, resource, issuer, and token-kind mismatches.

## Phase 3: Peer integration

- [x] [serial] r[molten.capability_token.peer_roles] Require subscriber, publisher, relay, sync, job-worker, node-control operator, and promotion roles to reference capability tokens or proofsets resolved at use time.
- [x] [parallel] r[molten.capability_token.no_generic_membership] Ensure generic capability tokens can support Raft membership requests but cannot replace membership preflight, quorum-safety, or commit receipts.
- [x] [parallel] r[molten.capability_token.subsystem_boundary] Preserve subsystem-specific provenance, source-gate, retention, execution, replay, consensus, and resource gates after capability admission passes.

## Phase 4: Tests and validation

- [x] [serial] r[molten.capability_token.positive_negative_tests] Add positive token admission fixtures and negative tests for bearer-only use, wrong holder, wrong session, wrong operation, over-broad scope, expired token, revoked issuer/delegation, caveat failure, missing policy/resource, token import as authority, and handoff/session/transport-as-token attempts.
- [x] [serial] r[molten.capability_token.validation] Run focused capability-token tests, peer/session/subscriber/promotion tests, authority tests, consensus boundary tests, formatting, and Cairn validation before archiving.
