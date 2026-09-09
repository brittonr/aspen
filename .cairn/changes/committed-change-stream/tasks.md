# Tasks

## Phase 1: Stream mechanics

- [ ] [serial] Define the committed-transition event schema, stable event identities, and cursor identity (service identity, stream epoch, revision, event ordinal); implement single-owner atomic state-plus-event commits. r[molten.observations.stream]
- [ ] [serial] Implement bounded-buffer resumable subscription, explicit retention gaps, and the snapshot-plus-watermark operation. r[molten.observations.stream]

## Phase 2: Publication eligibility

- [ ] [serial] Add the profile-parameterized eligibility gate between staging and metadata publication, with Nickel-authored profile requirements for local and replicated deployments. r[molten.observations.eligibility]
- [ ] [serial] Implement the eligibility receipt binding (content identity, operation, receiver set, retention generation, placement epoch, storage guarantee kind) and rejection of replica-count-only receipts. r[molten.observations.eligibility]

## Phase 3: Authorization and dedup

- [ ] [serial] Enforce that event, cursor, and stream possession authorize nothing; wire referenced-content reads through existing current capability checks. r[molten.observations.authorization]
- [ ] [serial] Implement stable-identity deduplication for replayed events. r[molten.observations.authorization]

## Phase 4: Tests

- [ ] [serial] Add positive tests: single-owner atomicity, restart resume, snapshot-plus-watermark completeness. r[molten.observations.stream]
- [ ] [serial] Add negative tests: gap forces resnapshot, stale-epoch cursor rejection, premature publication blocked, replica-count-only receipt rejected, capabilityless content read refused. r[molten.observations.eligibility] r[molten.observations.authorization]
- [ ] [serial] Extend the fault-conformance phases with subscriber restart after publication but before acknowledgement. r[molten.observations.stream]

## Phase 5: Validation

- [ ] [serial] Run workspace tests, strict Clippy, and `cairn validate --root .`; confirm no dataspace `Observe`, durable-delivery, or election/lease semantics changed. r[molten.observations.stream] r[molten.observations.eligibility]
