## Context

Coordination requests currently parse into service/operation/key/payload values, then preparation helpers clone and mutate candidate state or return denial diagnostics. Generated tests already check invariants like stale-token denial, FIFO ordering, duplicate no-advance, and no mutation on denial. The next step is to make those semantics the public pure core, not an implementation side effect of helper structure.

## Design

### Primitive transition cores

Each primitive gets a pure transition function or shared dispatch table shaped around:

```text
(current state, manifest limits, request, idempotency/replay facts, admission facts) -> transition result
```

The result includes:

- decision;
- next state for pass or preserved state for deny/replay;
- issued token or dequeued item facts;
- status assertion fact;
- receipt check names;
- diagnostics;
- commit/control-plane intent facts for the shell.

The core must not mutate runtime state in place, write receipts, append to ledgers, read clocks, read files, or touch Raft/Redb directly.

### Primitive coverage

Initial relation coverage includes:

- lock acquire/release with owner and fencing-token guards;
- queue enqueue/dequeue with FIFO and capacity guards;
- semaphore acquire/release with holder and capacity guards;
- rate-limit admit/deny with bounded counters;
- election grant/revoke or update with token and leader guards;
- barrier arrive/release/reset behavior with participant and threshold guards;
- registry register/read/update/remove with endpoint evidence guards.

### Duplicate replay

Operation-id replay should be represented as a transition kind that returns the prior receipt/output refs and preserves state. Conflicting duplicate operation ids deny and preserve state.

### Shell boundary

The shell applies a passing transition only after authority, policy, resource, idempotency, and control-plane commit evidence pass. Denied and duplicate transitions produce receipts without committing mutation. Dataspace assertions are published from transition output facts only after commit.

### Tests

Keep existing example tests and extend generated traces to cover every primitive's pass, denial, duplicate replay, and conflicting duplicate paths. Add assertions over before/after state refs for every denial.