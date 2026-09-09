# Committed-Change Stream Specification

## ADDED Requirements

### Requirement: Committed transitions are durably ordered per service

r[molten.observations.stream] The committed-change stream MUST record committed
service transitions only. Where one storage owner controls both state and event, it
MUST advance state, event, and stream position in one transaction. Where they span
independent stores, the coordination protocol MUST be explicit and the stream MUST
NOT imply cross-store atomicity.

#### Scenario: Single-owner commit is atomic

- GIVEN a service transition owned by one storage engine
- WHEN the transition commits
- THEN the state update, its event, and the advanced stream position MUST be
  observable together or not at all.

#### Scenario: Independent stores do not claim atomicity

- GIVEN a transition whose state and event live in independent stores
- WHEN the protocol records the transition
- THEN the stream entry MUST represent the committed transition explicitly and the
  documentation MUST NOT claim cross-store atomicity.

### Requirement: Cursors are resumable, bounded, and gap-explicit

r[molten.observations.stream] Stream cursors MUST be identified by
(service identity, stream epoch, revision, event ordinal) and MUST support resumable
subscription with bounded buffering. Retention gaps MUST be explicit, a subscriber
past a gap MUST resnapshot, and a snapshot-plus-watermark operation MUST exist such
that a subscriber taking a snapshot and starting from the watermark cannot miss a
committed change.

#### Scenario: Resume after restart loses nothing committed

- GIVEN a subscriber that observed events through cursor C before restarting
- WHEN it resumes from C
- THEN it MUST receive every committed event after C within retention, or an
  explicit gap signal requiring resnapshot.

#### Scenario: Snapshot plus watermark misses nothing

- GIVEN a subscriber that takes a snapshot and its watermark
- WHEN the stream delivers events from the watermark onward
- THEN no change committed after the snapshot is missing from the combined view.

#### Scenario: Epoch change rejects stale cursors

- GIVEN a stream epoch change
- WHEN a consumer presents a cursor from the prior epoch
- THEN the stream MUST reject it explicitly rather than silently remapping it.

### Requirement: Publication eligibility is profile-specific and receipt-bound

r[molten.observations.eligibility] The publication path MUST check a
profile-specific eligibility requirement between content staging and authoritative
metadata publication. The eligibility receipt MUST bind exact content identity,
operation, receiver set, retention generation, placement epoch, and storage
guarantee kind. A replica count alone MUST NOT satisfy the gate, and the requirement
MUST be a profile input so local and replicated deployments declare different
durability explicitly.

#### Scenario: Replicated profile blocks premature publication

- GIVEN a profile requiring replicated availability before publication
- WHEN publication is attempted without a conforming eligibility receipt
- THEN the gate MUST refuse publication and the head MUST NOT advance.

#### Scenario: Replica count alone is insufficient

- GIVEN an eligibility receipt naming only a replica count
- WHEN the gate evaluates it
- THEN the gate MUST reject it for missing content identity, generation, placement
  epoch, or storage guarantee bindings.

#### Scenario: Local profile differs explicitly

- GIVEN a local development profile and a replicated deployment profile
- WHEN their eligibility requirements are inspected
- THEN each MUST declare its own requirement as a recorded profile input.

### Requirement: Stream possession grants no content authority

r[molten.observations.authorization] Possession of an event, cursor, or stream
position MUST NOT grant access to referenced content; reading referenced content
MUST require the existing current capability checks. Consumers MUST deduplicate by
stable event identity.

#### Scenario: Cursor without capability cannot read content

- GIVEN a consumer holding a valid cursor for an event referencing content
- WHEN the consumer attempts to read the referenced content without a current
  capability
- THEN the content store MUST refuse the read.

#### Scenario: Replay is deduplicated

- GIVEN a resumed subscription that redelivers an already-observed event
- WHEN the consumer processes the stream
- THEN the stable event identity MUST allow exact deduplication of the replayed
  event.
