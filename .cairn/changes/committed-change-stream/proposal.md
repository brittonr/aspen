# Proposal: Durable committed-change stream and publication eligibility

## Why

The 2026-09-09 review of the sled architectural outlook found Molten's strongest
match to be the separation of bulk-data movement from metadata ordering — a
separation Molten already makes with `content-store.v1`, `content-exchange.v1`, and
coordination payloads that carry references rather than bytes. What is missing is the
explicit bridge in both directions:

1. **Publication eligibility.** No profile-level contract states when staged and
   verified content becomes eligible for authoritative metadata publication (for
   example, required replicated availability). A replica count alone is already
   documented as insufficient; the requirement must bind content identity,
   generation, placement epoch, and the storage guarantee.
2. **Committed observations.** Molten has dataspace `Observe`, a durable delivery
   extension, and a world-promotion outbox, but no committed-change stream an
   orchestrator can resume: reactive observation, durable work delivery, and
   committed-effect eligibility remain distinct guarantees without a durable,
   ordered record of committed transitions between them.

This change adds both, narrowly scoped, without introducing a second replication
stack or general-purpose broker, and without adopting the sled wiki's election or
lease proposals.

## What Changes

- Define a committed-change stream for committed service transitions (for example
  `JobSubmitted`, `TaskClaimed`, `ArtifactPublished`, `WorkflowCompleted`): where one
  storage owner controls state and event, both plus the stream position advance in
  one transaction; across independent stores, an explicit protocol is required and
  cross-store atomicity MUST NOT be implied. r[molten.observations.stream]
- Define resumable cursors scoped as (service identity, stream epoch, revision,
  event ordinal), bounded buffering, explicit retention gaps, and a
  snapshot-plus-watermark operation so subscribers cannot miss changes between
  snapshot and stream start. r[molten.observations.stream]
- Define a profile-specific publication-eligibility gate between content staging and
  metadata publication, receipt-bound to exact content, operation, receiver,
  generation, placement epoch, and storage guarantee. r[molten.observations.eligibility]
- Keep authorization current: possession of an event or cursor MUST NOT grant access
  to referenced content; consumers deduplicate by stable event identity.
  r[molten.observations.authorization]

## Impact

- New committed-transition stream surface beside the existing dataspace observation
  and durable delivery extensions; metadata schema and adapter for the stream.
- Publication path gains an eligibility check point parameterized by profile; local
  development and replicated profiles declare different requirements explicitly.
- No change to dataspace `Observe` semantics, durable delivery guarantees, or
  election/lease mechanisms.

## Out of Scope

- A general-purpose event broker, cross-service global ordering, or delivery of
  large payloads inside control-plane events.
- CRDT-style or last-write-wins merge of application state; map mechanics stay
  separate from semantic merge and branch authority.
- Discarding world snapshots or compacted-history dependencies on the sled
  log-remains-sufficient analogy; required reconstruction material may exist nowhere
  else.

## Affected Specs

- `committed-change-stream`: atomic commit-state/event binding, resumable cursors,
  snapshot-plus-watermark, profile-specific publication eligibility, and
  authorization non-claims.
