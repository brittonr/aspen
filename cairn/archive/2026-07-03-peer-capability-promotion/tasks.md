# Tasks: peer-capability-promotion

## Phase 1: Promotion model

- [x] [serial] r[molten.peer_promotion.record_model] Define canonical promotion request, promotion grant, promotion receipt, and demotion receipt records with target peer/session, role delta, scope, issuer, approvals, policy/resource, expiry, revocation, and evidence bindings.
- [x] [serial] r[molten.peer_promotion.role_delta] Implement pure role-delta validation from current admitted capabilities to requested capabilities with attenuation and scope checks.
- [x] [parallel] r[molten.peer_promotion.authority_separation] Enforce that promotion authority is separate from the target capability and cannot be satisfied by transport, session, handoff, subscription, or import receipts.

## Phase 2: Preflight/apply and cleanup

- [x] [serial] r[molten.peer_promotion.preflight_apply] Add dry-run promotion preflight and explicit apply flows that emit deterministic receipts and update peer-session read models only after gates pass.
- [x] [serial] r[molten.peer_promotion.node_apply_boundary] Update node-local peer session read models only after passing promotion apply or demotion receipts.
- [x] [serial] r[molten.peer_promotion.demotion_cleanup] Add demotion/revocation flow that narrows peer capabilities and retracts dependent subscriptions, live refs, handler bindings, queued jobs, and session state.
- [x] [parallel] r[molten.peer_promotion.approval_policy] Support optional approval refs or policy-selected multi-approval requirements for high-risk role transitions.
- [x] [parallel] r[molten.peer_promotion.apply_no_subsystem_side_effects] Keep promotion apply limited to session/capability state and require separate subsystem operations for sends, jobs, retention, sync, relay, or membership changes.

## Phase 3: Boundaries and diagnostics

- [x] [serial] r[molten.peer_promotion.no_self_escalation] Deny self-promotion, transitive escalation, over-broad target roles, revoked issuers, stale grants, and subscriber write-upgrades without matching promotion authority.
- [x] [parallel] r[molten.peer_promotion.subscriber_upgrade] Require subscriber/read-only peers to pass promotion before gaining publish/assert/retract/relay/import capabilities.
- [x] [parallel] r[molten.peer_promotion.raft_boundary] Ensure generic peer promotion cannot produce Raft voter, non-voter, learner, or linearizable-read roles without the separate membership/read-index gates.
- [x] [parallel] r[molten.peer_promotion.diagnostics] Add diagnostics that explain current role, requested role, admitted/denied delta, missing promotion authority, revocation/expiry, approvals, and next action.

## Phase 4: Tests and validation

- [x] [serial] r[molten.peer_promotion.positive_negative_tests] Add positive scoped promotion/demotion tests and negative tests for self-promotion, missing promotion authority, stale grant, revoked issuer, over-broad target, transitive escalation, subscriber write-upgrade, handoff-as-promotion, and Raft membership promotion.
- [x] [serial] r[molten.peer_promotion.validation] Run focused promotion/demotion tests, peer-session/subscriber tests, authority tests, consensus boundary tests, formatting, and Cairn validation before archiving.
