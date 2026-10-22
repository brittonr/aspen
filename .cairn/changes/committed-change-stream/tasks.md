# Tasks: Durable committed-change stream and publication eligibility

## Phase 1: Stream mechanics

- [x] [serial] Define the committed-transition event schema, stable event identities, and cursor identity (service identity, stream epoch, revision, event ordinal); implement single-owner atomic state-plus-event commits. r[molten.observations.stream]
      The core module `committed_stream` carries the event, the cursor, the retained stream, and the commit input. One
      commit advances the state reference, the events, and the stream position together, and each event carries a
      stable identity over the service, epoch, revision, ordinal, and payload reference.
- [x] [serial] Implement bounded-buffer resumable subscription, explicit retention gaps, and the snapshot-plus-watermark operation. r[molten.observations.cursors]
      The subscription is bounded by an explicit buffer bound. A cursor behind the retained window reports an explicit
      retention gap, a cursor from a prior epoch is rejected, a cursor for another service is refused, and the
      snapshot-plus-watermark operation returns the position after the last committed event.

## Phase 2: Publication eligibility

- [x] [serial] Add the profile-parameterized eligibility gate between staging and metadata publication, with Nickel-authored profile requirements for local and replicated deployments. r[molten.observations.eligibility]
      The requirement is the profile's replica policy: the minimum verified replicas and the minimum fault domains the
      deployment must observe. A local profile and a replicated profile therefore declare different durability
      explicitly. The gate sits between the transfers and the metadata publication of the replication shell.
- [x] [serial] Implement the eligibility receipt binding (content identity, operation, receiver set, retention generation, placement epoch, storage guarantee kind) and rejection of replica-count-only receipts. r[molten.observations.eligibility]
      The receipt binds every field and carries a canonical identity over them. A receipt that names only a replica
      count is refused: the storage guarantee must be named, and a local guarantee never satisfies a profile that asks
      for replicated availability across fault domains.

## Phase 3: Authorization and dedup

- [x] [serial] Enforce that event, cursor, and stream possession authorize nothing; wire referenced-content reads through existing current capability checks. r[molten.observations.authorization]
      The stream references the committed payload by content identity and never inlines it, and the stream API grants no
      access to the referenced content. Reading that content still requires the existing current capability checks.
- [x] [serial] Implement stable-identity deduplication for replayed events. r[molten.observations.authorization]
      A replayed event keeps one identity, and the deduplication keeps the first occurrence in commit order and reports
      the repeated identities.

## Phase 4: Tests

- [x] [serial] Add positive tests: single-owner atomicity, restart resume, snapshot-plus-watermark completeness. r[molten.observations.stream] r[molten.observations.cursors]
- [x] [serial] Add negative tests: gap forces resnapshot, stale-epoch cursor rejection, premature publication blocked, replica-count-only receipt rejected, capabilityless content read refused. r[molten.observations.eligibility] r[molten.observations.authorization] r[molten.observations.tests]
      The stream fixtures cover the resume, the snapshot watermark, the retention gap, the stale epoch, the foreign
      service, the bounded subscription, the invalid commit, and the deduplication. The gate fixtures cover the
      replicated requirement, the local profile, the replica-count-only refusal, the incomplete observation, and the
      receipt identity. The shell fixture shows the bound receipt and the refusal to claim content the profile cannot
      admit.
- [ ] [serial] Extend the fault-conformance phases with subscriber restart after publication but before acknowledgement. r[molten.observations.stream]
      The fault-conformance phases live in the world-fault conformance profile, which exercises the world's
      reconciliation rather than a subscriber of the committed-change stream. The subscriber restart is covered by the
      resume fixture, which resumes from a cursor after the producer committed further events. The phase stays open
      until the conformance profile gains a committed-change subscriber role.

## Phase 5: Validation

- [x] [serial] Run workspace tests, strict Clippy, and `cairn validate --root .`; confirm no dataspace `Observe`, durable-delivery, or election/lease semantics changed. r[molten.observations.stream] r[molten.observations.eligibility]
