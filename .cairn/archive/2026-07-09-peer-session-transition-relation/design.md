## Context

Peer session records are a boundary between transport reachability and Molten authority. The accepted peer specs already require explicit lifecycle transitions and authority separation, but implementation should expose a single transition relation that reviewers can audit and property tests can exhaustively exercise.

## Design

### Transition core

Introduce a pure peer-session transition core with inputs shaped around:

- prior `PeerSession` or canonical before-state ref;
- requested `PeerSessionEvent` such as invite, handshake-start, negotiation-pass, admit, connect, expire, revoke, quarantine, recover;
- target state from the reviewed relation;
- observed topic, endpoint, freshness tick, revocation facts, bootstrap admission facts, capability facts, authority facts, policy/resource refs, and replay/idempotency refs.

The core returns a transition result containing:

- decision;
- diagnostics;
- after-state for pass or preserved before-state for deny;
- canonical receipt input facts;
- guard check names and evidence refs.

No filesystem, network, clock, random, process, Redb, tracing, receipt persistence, or live-Iroh access belongs in the core.

### Reviewed relation

Keep the transition table data-oriented and auditable. Valid progression covers discovery/invitation, handshake, negotiation, admission, connection, expiry, revocation, quarantine, and explicitly admitted recovery. Error and terminal exits require a named event and recovery/admission evidence; transport observations alone never advance a terminal or quarantined state.

### Receipts

Transition receipts should bind:

```text
peer/session id
from-state
requested event
target-state
decision
before-state ref
after-state ref or preserved-state ref
guard evidence refs
diagnostics
checks
```

Receipt parsing should recompute the transition decision from receipt-bound inputs where feasible and should reject stale or mismatched before/after bindings.

### Tests

Use deterministic unit tests for named examples and bounded generated traces for relation coverage. Negative tests should include discovered-to-connected skips, wrong topic, missing bootstrap, missing authority, stale ticket, revoked profile, terminal-state exit without recovery, quarantine bypass by transport observation, and transport-as-authority attempts.