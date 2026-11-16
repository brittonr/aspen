## Context

`prepare_rate_acquire` increments `state.rates[key]` and denies at `manifest.rate_limit`. Nothing decrements or clears the count, and neither the manifest nor the coordination state carries time.

## Decisions

### Decision: Name the mechanism a quota rather than add a window

**Choice:** Keep the counting rule and document it as a per-key lifetime quota in code, the protocol reference and the specification. The denial diagnostic names the quota and that it does not reset.

**Rationale:** A rate window needs a deterministic logical clock committed with coordination state. Letting requests advance that clock would let a limited client reset its own limit, and coordination has no admitted time authority today. The issue accepts naming the mechanism a quota; that removes the misleading claim without inventing authority.

## Risks / Trade-offs

- Callers that expected recovery over time must provision a new key or a new coordination state; the denial now says so.
- The `rate-limit` wire name remains for manifest and receipt compatibility.
