# Proposal: Own remote assertions per session

## Why

Remote dataspace envelopes are applied through the local turn boundary, and the delivery path validates declared refs
before it applies an envelope. No source states which owner holds an assertion that arrived from a peer, and the remote
dataspace module has no disconnect or session-close cleanup path.

The manual ties assertion lifetime to its owner: an assertion exists only while its actor maintains it, and the
dataspace forwards the withdrawal to subscribers (`07-syndicated-actor-model.md → Dataspaces`). The accepted requirement
`molten.runtime_spine.reference_lifetimes` already requires that session close makes session-scoped references invalid
and cleans up dependent assertions, subscriptions, and pending operations. The tracey baseline lists it as
implementation-unestablished.

Without a stated owner, a peer's fact can outlive the peer session that asserted it, and a reconnect can leave stale
facts next to fresh ones. This package decides the rule and tests it.

## What Changes

- Name the owner: a remote assertion MUST be owned by the receiving session scope, recorded in the applied assertion
  record, so the owner is readable from evidence. r[molten.runtime_spine.remote_assertion_ownership]
- On session close or disconnect, session-owned remote assertions MUST retract through the normal owner-scope cleanup
  path, and matching observers MUST receive the retraction before the session identity can be reused.
- An envelope whose declared owner is unknown or belongs to a closed session MUST deny before it applies, so cleanup
  cannot be undone by a late delivery.
- Reconnection MUST require the peer to assert again. No reconnect path may resurrect a previous session's assertions
  from a replay record, and replayed deliveries for a closed session stay diagnostic.

## Impact

- **Files**: `src/remote/parts/dataspace/`, `src/runtime/dataspace/state.rs`, `docs/architecture.md` remote dataspace
  section, and the referenced tests.
- **Testing**: positive cases for disconnect retraction and observer notification, and for a re-assert after reconnect;
  negative cases for a late delivery to a closed session, an unknown declared owner, and a replay that tries to
  resurrect a closed session's assertions.
- **Non-goals**: no delivery-completeness claim, no reconnect-time fact restoration, no change to transport receipts or
  authority, and no Syndicate wire compatibility.
