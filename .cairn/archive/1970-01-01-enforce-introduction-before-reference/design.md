# Design: Enforce introduction before reference

## Context

Remote dataspace envelopes carry actions and payload refs, and the delivery path validates declared content refs before
it applies an envelope to the local turn boundary. Session state carries generation fencing. Nothing derives which
references a session has introduced, so an envelope can carry a reference that the peer never saw established.

## Approach

Keep no separate introduction ledger. Derive the introduced set as a pure function over session state:

- Start from the session's established bootstrap references: the endpoint identity refs and session refs bound during
  bootstrap.
- Add every reference that appears inside the payload of a live assertion owned by that session.
- Remove a reference when the last live assertion that mentions it retracts.

A message check is then a pure membership test against that derived set. Receiver-side denial happens before delivery,
inside the existing admission path, so it inherits the current denial evidence and rollback behavior. Sender-side
refusal happens in the envelope build step and emits the same denial evidence class.

Lifetime follows the manual's `WireSymbol` rule: the introduction lives exactly as long as some live assertion mentions
the reference (`08-protocol.md → Membranes`). Session close removes the session's assertions, so the derived set
empties with no extra cleanup step.

## Decisions

### Decision: Derive introductions from live assertions instead of storing a counter

**Choice:** Compute the introduced set from live assertions on each check.

**Rationale:** A stored counter is a second source of truth that must stay consistent with assertion retraction,
session close, and turn rollback. A derivation cannot drift, and it can be cached per session when a profile needs the
optimization.

### Decision: Denial evidence, not an error reply

**Choice:** Record canonical denial evidence on the receiver and refuse on the sender. Do not add a protocol error
message.

**Rationale:** The manual itself makes the underlying failure model silent and permits debugging aids only
(`07-syndicated-actor-model.md`, note 6). Molten's non-authority receipt boundary already covers this case.

## Risks / Trade-offs

- Bootstrap references must be defined precisely, or a legitimate first message can deny. The bootstrap path already
  binds endpoint and session refs, so this change consumes those values rather than minting new ones.
- Deriving the set on each message costs a scan of live assertions. Bound the scan by the session's assertion count and
  cache per committed turn if a profile needs it.
- Existing recorded fixtures may carry unintroduced references. Pre-change fixtures are re-recorded with an explicit
  introducing assertion, and the change records which fixtures moved.
