## Context

Molten's architecture includes coordination primitives as control-plane services. They should be backed by the Raft/control registry or another explicitly admitted strong-consistency state machine, then reflected into the dataspace as facts. They must not turn every actor message into a consensus command.

## Goals

- Define `coordination-service-manifest-v1`, `coordination-request-v1`, `coordination-receipt-v1`, `fencing-token-v1`, and `coordination-status-assertion-v1`.
- Implement first primitives over the Raft control registry state machine: lock/lease with fencing, FIFO queue, semaphore, rate limit, election, barrier, and service registry pointer.
- Use operation-id/idempotency receipts for all mutating requests.
- Gate each operation through authority, policy, resource, and read-index/commit evidence.
- Publish committed state as dataspace assertions for observers.
- Emit denial receipts for stale fencing tokens, expired leases, duplicate acquisition, queue overflow, semaphore exhaustion, and unauthorized operations.

## Non-Goals

- No global actor-message ordering.
- No coordination from local wall-clock without logical/lease policy evidence.
- No distributed lock safety claim without committed fencing token receipts.
- No implicit authority from owning the client session id.

## Records

```preserves
<coordination-request-v1 "molten.coordination.request.v1"
  <service "lock"|"queue"|"semaphore"|"rate-limit"|"election"|"barrier"|"registry">
  <operation "acquire"|"release"|"enqueue"|"dequeue"|"elect"|"read">
  <key "resource:name">
  <client-session <session-id>>
  <operation-id <operation-ref>>
  <authority [<authority-context-ref> ...]>
  <resource [<resource-ref> ...]>
  <policy [<policy-ref> ...]>
  <checks [<check "control-plane-command" "pass"> ...]>>
```

```preserves
<coordination-receipt-v1 "molten.coordination.receipt.v1"
  <decision "pass"|"deny">
  <service "lock">
  <operation "acquire">
  <request <request-ref>>
  <raft <commit-or-read-receipt-ref>>
  <token <fencing-token-ref-or-none>>
  <state <coordination-state-ref>>
  <dataspace [<assertion-ref> ...]>
  <diagnostics ["..." ...]>
  <checks [<check "fencing-token-monotonic" "pass"> ...]>>
```

## Dataspace Reflection

After commit/apply, coordination state is reflected as assertions such as:

- `<lock-held key token owner expiry>`
- `<queue-depth key n>`
- `<semaphore-available key n>`
- `<leader key token owner>`
- `<service-registered name endpoint-ref evidence-ref>`

These assertions are observational facts; mutating coordination still goes through the control-plane state machine.
