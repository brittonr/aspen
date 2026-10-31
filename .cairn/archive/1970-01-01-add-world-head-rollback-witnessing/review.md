# Implementation review: world-head rollback witnessing

## Disposition

The independent-witness profile, its pure core, shell, recovery, receipts and the
same-host consumer pilot are complete for the experimental single-provider profile.
This is an implementation review, not independent security approval, production
readiness or release authority. Executed commands and results are in
`evidence/2026-10-30-verification.md`; the provider decision is in
`evidence/2026-10-30-provider-decision.md`.

## Audit of the inherited implementation

Every task was audited against source before closure; each task line names the
implementing symbols and tests. The audit found one requirement gap and fixed it.

### Gap: orphan recovery did not compose with the external index ordinal

The shell reserved the external claim at the Molten head generation
(`after.generation`, always `before + 1`). The pinned provider keeps an orphaned
reservation as its high-water after `ResolveOrphan` and rejects any reservation at or
below it. After one orphan, every later strong advance on that branch would request
the abandoned ordinal and be rejected forever. That contradicts the design
("witness-first flow can leave harmless orphan claims"), the reconciliation outcome
`superseded`, and the operator documentation, which already required recovery at a
strictly higher generation. Release-channel's own contract states that a fresh signed
query proving a resolved-orphan high-water is the only way a successor may skip.

Fix: the external ordinal is decoupled from the head generation.
`provider::ordinal::plan` derives it from a fresh nonce-bound signed query: an exact
pending reservation is recovered unchanged; otherwise the ordinal is the retained
external high-water plus one with the exact committed predecessor. A high-water above
the committed claim is accepted only as a validated resolved orphan extending that
predecessor; unresolved reservations/orphans, reuse of the abandoned digest, a stale
(rolled-back) predecessor and forged gaps fail closed. The head generation still
advances by exactly one and remains bound in the claim digest, so the core,
receipts and highest-state codec are unchanged. `recover_reservation` now matches the
exact claim digest and predecessor and keeps the provider-signed ordinal, so a lost
response across a gap recovers the original reservation instead of recomputing it.
Current reads already used the durably stored reservation claim.

Resolution itself stays an explicit operator action; Molten never auto-resolves an
orphan, and resolution grants no branch authority.

## Reviewed invariants

- Stage precedes external dispatch; the final local CAS rereads head, durable stage,
  retained highest, policy and authority in one Redb transaction.
- A reserved provider state is recorded as `reserved`; only a confirmed external
  commit yields `admitted` currentness.
- Lost reserve/commit/orphan responses return `Uncertain` and keep the exact
  reservation; recovery observes before acting and never dispatches a fresh Reserve.
- Receipts reject witness-as-authority, collapsed role references, local-profile
  independent claims and dropped non-claims.
- The pilot's rollback verdict now requires an authenticated, strictly newer external
  claim distinct from the local head's reservation; any other read failure stays an
  error. Each pilot step must print its exact expected outcome.

## Non-claims

- Same-host provider and consumer share one administrative and physical failure
  domain. The pilot is experimental, uses fixed nonproduction keys, and does not
  establish independent witnessing, provider honesty, fork freedom, global consensus,
  production trust or release eligibility.
- The abandoned competitor in the pilot is an explicit external reservation that the
  consumer never staged; it models a writer whose local CAS did not commit, not a
  concurrent multi-process race.
- No quorum beyond one provider was executed against a real provider; multi-witness
  quorum is covered only by pure-core fixtures.
