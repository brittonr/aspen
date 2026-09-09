# Design: Durable committed-change stream and publication eligibility

## Context

Molten separates content transfer from metadata publication but has no explicit
eligibility contract between them and no durable, resumable record of committed
transitions. The three existing observation-adjacent guarantees are distinct and
stay distinct: dataspace `Observe` (reactive), durable delivery (work delivery), and
the world-promotion outbox (committed-effect eligibility). This change adds a fourth,
narrow surface: a committed-change stream over committed service transitions.

## Goals

- An orchestrator can rebuild its scheduling view from a snapshot plus committed
  changes instead of transient notifications or repeated full scans.
- Profiles state their own durability requirement before metadata publication; a
  local development profile and a replicated deployment cannot silently claim the
  same durability.
- No cross-store atomicity is implied where none exists.

## Non-Goals

- Global ordering across unrelated services; cursors order only within
  (service, epoch).
- A second replication stack, a broker, or adoption of the sled wiki's election or
  lease protocol proposals.
- Making any world snapshot or retained root discardable.

## Approach

### Committed-change stream

Events represent committed service transitions only — never speculative callbacks.
Where a single storage owner controls both state and event, the owner updates state,
appends the event, and advances the stream position in one transaction. Where they
span independent stores, the protocol is explicit and the stream carries only the
committed-transition fact; consumers tolerate replay via stable event identities.

Cursor identity is `(service identity, stream epoch, revision, event ordinal)`. A
stream epoch change invalidates prior cursors explicitly. The contract includes
bounded buffering, explicit retention gaps (a consumer past a gap MUST resnapshot),
and a snapshot-plus-watermark operation returning a consistent view and the stream
position from which no committed change is missed.

Events carry stable identities for deduplication and reference payloads by content
identity, never inline bytes.

### Publication eligibility

Insert a profile-parameterized gate in the publication path:

```
stage content -> verify canonical identities
             -> establish protection and storage observations
             -> check profile eligibility requirement
             -> publish metadata reference
             -> release committed observations
```

The requirement is a profile input (Nickel-authored), not a hard-coded constant. The
eligibility receipt binds exact content identity, operation, receiver set, retention
generation, placement epoch, and the storage guarantee kind. A replica count alone
never satisfies the gate. The gate sits above the existing consistency port; no
consensus mechanism changes.

### Authorization

Stream and cursor possession authorize nothing. Reading an event's referenced
content requires the existing current capability checks. Deduplication uses stable
event identities and never substitutes for authorization.

### Relationship to `harden-prolly-publication-integrity`

Eligibility observations reuse the phase-aware classification and readback
vocabulary from that change; the stream appends only after the publication outcome
is committed or explicitly resolved, never while unknown.

## Testing

- Positive: atomic state-plus-event under a single owner; resume from cursor across
  restart; snapshot-plus-watermark with no missed change.
- Negative: gap handling forces resnapshot; epoch change rejects stale cursors;
  eligibility gate rejects replica-count-only receipts; cursor possession without
  capability cannot read referenced content.
- Crash coverage at the existing fault-conformance phases: subscriber restart after
  publication but before acknowledgement.

## Risks

- Duplicated observation surfaces can drift; the design keeps each guarantee in its
  owning extension and documents the boundaries.
- Epoch and retention semantics can silently drop events; explicit gaps and the
  resnapshot requirement are the control.
