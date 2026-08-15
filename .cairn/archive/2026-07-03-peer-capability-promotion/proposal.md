## Why

Peers will need controlled upgrades: subscriber to publisher, read-only observer to sync participant, worker candidate to admitted executor, or operator peer to a scoped node-control role. Those upgrades must be explicit capability promotions, not side effects of being connected, subscribed, useful, or holding a handoff bundle. Promotion authority itself must be scoped, attenuated, expiring, revocable, and auditable so a peer cannot self-promote or amplify privileges transitively.

## What Changes

- Define canonical promotion request, promotion grant, promotion decision receipt, and demotion/revocation receipt records.
- Require promotion to evaluate a role delta from current admitted capabilities to requested capabilities with monotonic attenuation and explicit anti-escalation checks.
- Add dry-run promotion preflight and apply flows: preflight explains what would change, apply mutates session/read-model state only after matching promotion authority, policy, resource, expiry, revocation, and optional approval evidence pass.
- Support demotion as the safer inverse path, retracting dependent subscriptions, grants, live refs, jobs, and sessions.
- Keep Raft/control-plane membership promotion on the stronger membership-admission path, not the generic peer promotion path.

## Impact

- **Files**: authority model specs, peer/session lifecycle specs, subscriber role specs, node read-model updates, CLI promotion UX, diagnostics, and positive/negative tests.
- **Testing**: positive scoped promotion/demotion tests and negative tests for self-promotion, transitive escalation, missing promotion authority, stale grant, over-broad target role, revoked issuer, subscriber write-upgrade without promotion, and Raft membership promotion attempts.
