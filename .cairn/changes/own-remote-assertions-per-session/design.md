# Design: Own remote assertions per session

## Context

Delivered envelopes carry canonical action records and refs, and delivery applies them through the local turn
boundary. Sessions already carry identity and generation fences, and owner-scope cleanup already retracts assertions,
observers, and messages for one owner. The missing decision is which owner a remote assertion receives.

## Approach

Choose the session as the owner, and make the choice checkable:

- Apply a remote assertion with the receiving session scope as its owner, and record the owning session ref in the
  applied assertion record.
- Reject an envelope whose declared owner is unknown or belongs to a closed session, before the turn stages anything.
- Extend session close to run the existing owner-scope cleanup for the session scope, so assertions, observers, and
  pending operations retract together and candidate observers receive the retraction.
- Treat replayed deliveries for a closed session as diagnostic only; a reconnect requires a fresh assertion from the
  peer.

## Decisions

### Decision: The session owns remote assertions

**Choice:** Session-scoped ownership rather than receiver-global ownership.

**Rationale:** It matches the manual's fate-sharing rule and makes disconnect a meaningful retraction. Receiver-global
ownership would leave a departed peer's facts live with no event to end them, and would need a separate garbage
collector that duplicates the owner-scope rule.

### Decision: No resurrection on reconnect

**Choice:** A reconnect requires re-assertion; replay records never re-establish authority-bearing state.

**Rationale:** Resurrection would let a stale fact return without a current owner, which contradicts the lifetime rule
and creates a stale-fact window during restart.

## Risks / Trade-offs

- A fact a peer intended to be durable now dies with its session. The peer re-asserts after reconnect, and a durable
  local fact belongs in local state rather than in a remote assertion.
- Session close becomes a retraction path with observer fan-out. The existing cleanup already performs this work for
  local owners, so the change reuses one mechanism rather than adding a second.
- Recorded delivery fixtures that reuse a session identity after close must be re-recorded, and the change lists which
  fixtures moved.
