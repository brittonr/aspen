# Design: capability token spine

## Scope

This change defines the cross-cutting capability token model used by peer sessions, subscribers, promotions, node-control authority, remote dataspace, jobs, sync, retention, and future Basalt/UCAN integration. It does not replace subsystem admission gates; it makes the authority input to those gates uniform and explicit.

## Token records

`capability-token-v1` is a canonical value with:

- token kind and schema version,
- issuer and holder/principal refs,
- optional peer/session/actor/service binding refs,
- resource and ability/operation,
- scope and attenuation,
- caveats and required predicate/evidence refs,
- not-before/expiry/freshness refs,
- revocation refs and key-currentness refs,
- policy refs and resource refs,
- delegation/proof chain refs,
- subject content ref and evidence refs.

`capability-proofset-v1` groups one or more token/proof refs for a requested action and records intended holder, context, resource, ability, scope, and evidence refs. It may wrap local deterministic grants or future Basalt/UCAN proof bundles.

`capability-admission-receipt-v1` records the result of resolving a proofset for one requested action. It binds request ref, holder/session, resource, ability, scope, selected token refs, caveat decisions, policy/resource decisions, revocation/currentness, diagnostics, and pass/deny.

## Admission law

Tokens are not bearer-only. Admission must verify possession plus binding to the current holder/session/context, exact resource/ability/scope, attenuation, caveats, expiry, revocation, key-currentness, policy, resource envelope, and subsystem constraints. Importing a token only creates an evidence candidate; it does not mint current authority.

## Token taxonomy

Molten distinguishes:

- identity refs: bind identity but grant no authority,
- transport receipts: prove delivery/observation only,
- peer sessions: summarize admitted state but are not operation authority,
- handoff bundles: carry token candidates but are not tokens,
- bootstrap tickets/admissions: admit peer reachability/scope only,
- read/subscriber tokens: authorize egress/projection only,
- write/publish tokens: authorize mutation of declared surfaces,
- promotion tokens: authorize a capability delta,
- authority tokens: authorize side-effecting operation classes,
- membership evidence: supports but does not replace Raft membership admission.

## Basalt/UCAN path

Local deterministic token fixtures are allowed for tests and early implementation. The verifier must keep a replacement seam where Basalt/UCAN proofs, caveats, and revocation evidence can provide the token/proofset inputs without changing downstream subsystem gate semantics.

## Functional core

The pure core parses token/proofset values, computes admission decisions, validates caveats/attenuation/revocation inputs supplied in memory, and returns diagnostics. Shell code owns ledger reads, state-root import, clock/freshness source acquisition, Basalt/UCAN calls, and receipt persistence.

## Non-goals

- No ambient identity authority.
- No bearer-only capability tokens.
- No token admission from transport, peer session, handoff, import, or log receipt alone.
- No bypass of subsystem gates for provenance, source-gate, retention, execution, consensus, or resource policy.
