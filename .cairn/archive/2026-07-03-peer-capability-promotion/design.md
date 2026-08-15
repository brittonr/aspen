# Design: peer capability promotion

## Scope

This change makes peer role upgrades and downgrades explicit. It covers promotion between ordinary peer/session roles such as subscriber, publisher, remote dataspace participant, federation participant, job worker candidate, job worker, and scoped node-control operator. It does not cover Raft voter/non-voter/learner membership; that remains under the stronger membership-admission path.

## Promotion records

`peer-capability-promotion-request-v1` names the target peer/session, current role evidence, requested role/capability set, requested scope, requester, reason, expiry, and supporting evidence.

`peer-capability-promotion-grant-v1` is the authority-bearing artifact. It names issuer, target peer/session, promotable-from roles, promotable-to roles, allowed scopes, attenuation, expiry, approval refs, policy refs, resource refs, revocation refs, and delegation chain refs.

`peer-capability-promotion-receipt-v1` records preflight/apply decision, prior session ref, requested delta, admitted delta, denied capabilities, diagnostics, and resulting session ref if applied.

`peer-capability-demotion-receipt-v1` records removal or narrowing of peer capabilities, dependent cleanup/retraction refs, diagnostics, and resulting session state.

## Promotion law

Promotion validates a role delta, not just a target role. The target capability set must be within the promotion grant's permitted `from -> to` transition and scope. Promotion authority is separate from the resulting capability. The promotion grant must be current, unrevoked, policy-admitted, resource-admitted, and bound to the target peer/session. Optional approval evidence may be required for high-risk transitions.

## Anti-escalation checks

Promotion denies when:

- the peer attempts self-promotion without an explicit self-promotion grant,
- the issuer lacks promotion authority for the requested transition,
- the requested target role exceeds the grant's allowed scopes or attenuation,
- a read-only/subscriber grant is used as promotion authority,
- a handoff bundle/import receipt is used as promotion authority,
- revocation, expiry, or key-currentness checks fail,
- the promotion would imply Raft membership, destructive retention, authority delegation, or source-gate/provenance trust without a subsystem-specific gate.

## Demotion and cleanup

Demotion is first-class and should be easier to admit than promotion. Demotion receipts revoke or narrow capabilities and trigger cleanup of dependent subscriptions, live refs, handler bindings, queued jobs, and read-model session state. Demotion does not delete historical evidence.

## CLI UX

`molten peer promote preflight` explains the proposed delta. `molten peer promote apply` applies an admitted promotion. `molten peer demote` narrows or removes capabilities. `molten peer promotion status` renders active promotion grants, pending requests, revocations, and dependent sessions.

## Functional core

The pure core computes role deltas, validates promotion grants, applies attenuation and revocation checks, and produces promotion/demotion decisions from in-memory values. Shells own ledger reads, state-root writes, live cleanup, and operator rendering.

## Non-goals

- No implicit promotion from repeated successful reads/writes.
- No promotion from transport identity, peer session state, handoff import, or subscription receipt alone.
- No generic promotion into Raft membership, destructive retention authority, or source-gate/provenance trust.
- No automatic transitive delegation beyond explicitly attenuated promotion grants.
