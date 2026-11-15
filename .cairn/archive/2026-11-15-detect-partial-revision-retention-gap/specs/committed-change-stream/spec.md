# Committed Change Stream Delta

## MODIFIED Requirements

### Requirement: Cursors are resumable, bounded, and gap-explicit
r[molten.observations.cursors] Stream cursors MUST be identified by
(service identity, stream epoch, revision, event ordinal) and MUST support resumable
subscription with bounded buffering. Retention gaps MUST be explicit, a subscriber
past a gap MUST resnapshot, and a snapshot-plus-watermark operation MUST exist such
that a subscriber taking a snapshot and starting from the watermark cannot miss a
committed change.

Clients MUST obtain one consistent selected-run snapshot and watermark, validate service/epoch, and read bounded pages. Missing retained history MUST require resnapshot. Artifact unavailability MUST remain distinct from retained-history gaps. Disconnect MUST NOT cancel execution.

#### Scenario: Retention overflow
- GIVEN a selected-run observer holds a cursor whose events have been evicted from the bounded retained stream
- WHEN the inclusive cursor precedes retained history
- THEN the server reports explicit resnapshot requirements rather than silently advancing.

#### Scenario: Resume after restart loses nothing committed

- GIVEN a subscriber that observed events through cursor C before restarting
- WHEN it resumes from C
- THEN it MUST receive every committed event after C within retention, or an
  explicit gap signal requiring resnapshot.

#### Scenario: Restart after publication but before acknowledgement

- GIVEN a subscriber checkpoint before a committed publication and a delivered
  page whose next cursor has not been acknowledged
- WHEN the deterministic fault-conformance subscriber restarts at `before-response`
  and restores only its acknowledged checkpoint
- THEN every published event MUST be redelivered within retention, including the
  unacknowledged page, and stable-identity deduplication MUST preserve the exact
  committed sequence
- AND missing, crossed, reordered or falsely acknowledged evidence MUST prevent a
  passing conformance receipt.

#### Scenario: A bounded page ends inside a multi-event revision

- GIVEN several committed events share one revision and a page ends before its
  last ordinal
- WHEN the subscriber resumes from the returned next-event cursor
- THEN it MUST receive the next ordinal rather than skip it.

#### Scenario: Snapshot plus watermark misses nothing

- GIVEN a subscriber that takes a snapshot and its watermark
- WHEN the stream delivers events from the watermark onward
- THEN no change committed after the snapshot is missing from the combined view.

#### Scenario: Epoch change rejects stale cursors

- GIVEN a stream epoch change
- WHEN a consumer presents a cursor from the prior epoch
- THEN the stream MUST reject it explicitly rather than silently remapping it.

#### Scenario: State-only transition preserves retained history

- GIVEN a durable progress journal and its current single-owner state binding
- WHEN a controller transition changes only pending/running observations or logical time and emits no accepted-output or terminal-run event
- THEN the owner MUST atomically bind the new state to the unchanged retained journal and watermark
- AND a subsequent snapshot MUST report the new state reference without rewriting or revalidating the unchanged event payloads.

#### Scenario: Partial revision retention cannot hide an ordinal gap

- GIVEN a subscriber consumed ordinal 0 of a three-event revision and retained the
  returned inclusive next-event cursor at ordinal 1
- WHEN a later 4,095-event revision causes retention to remove ordinals 0 and 1
  while ordinal 2 remains in the oldest retained revision
- THEN resuming from ordinal 1 MUST return an explicit retention gap requiring a
  fresh snapshot, rather than silently returning ordinal 2.

#### Scenario: The retained boundary remains inclusive

- GIVEN retention has advanced within a committed revision
- WHEN the cursor names the first retained ordinal or a later retained ordinal
- THEN subscription MUST return that event within the existing page bound
- AND an empty or untrimmed stream MUST continue accepting its start cursor.

