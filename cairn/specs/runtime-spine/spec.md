# Runtime Spine Specification

## Purpose

Defines the `runtime-spine` capability.

## Requirements

### Requirement: Canonical retention pins and classes
r[molten.retention.pin_model] Molten MUST represent retention pins as canonical records that bind object ref, object kind, pin source, reason, owner, expiry, policy refs, and evidence refs.

#### Scenario: Pin binds object and owner
- GIVEN an object ref and an owner with retention authority
- WHEN the object is pinned
- THEN the pin record binds the object, owner, source, policy refs, and evidence refs

r[molten.retention.classes] Molten MUST classify retained objects as ephemeral cache, debug trace, replay snapshot, audit receipt, durable value, public artifact, private secret ref, upgrade rollback, or legal hold.

#### Scenario: Unknown class is rejected
- GIVEN a retention request naming an unsupported class
- WHEN a pin or GC decision is evaluated
- THEN the request fails closed before changing retention state

r[molten.retention.no_name_gc] Molten MUST NOT treat mutable names, aliases, tags, or channels as sufficient proof that content is safe to delete.

#### Scenario: Name absence is not GC evidence
- GIVEN an object with no current human-readable name
- WHEN GC eligibility is evaluated
- THEN eligibility still depends on the reference index, pins, policy refs, and evidence refs

r[molten.retention.receipts] Molten MUST emit canonical receipts for pin, unpin, retain, eligibility, delete, tombstone, redact, compact, and denial decisions.

#### Scenario: Decision receipt records diagnostics
- GIVEN a retention decision that is denied
- WHEN the receipt is emitted
- THEN the receipt records the action, object ref, reference index ref, and denial diagnostics

### Requirement: Reference-index based GC eligibility
r[molten.retention.reference_index] Molten MUST build a bounded reference index covering active sessions, artifacts, blobs, receipts, snapshots, transcripts, docs, policies, upgrades, storage refs, remote cache refs, and evaluation cache refs before GC decisions.

#### Scenario: Index binds active pins
- GIVEN an object with active retention pins
- WHEN the reference index is built
- THEN the index lists those pin refs and binds the target object ref

r[molten.retention.gc_eligibility] Molten MUST deny GC unless the reference index proves no active or retained dependency remains.

#### Scenario: Incomplete proof denies deletion
- GIVEN an incomplete reference proof for an object
- WHEN deletion is requested
- THEN deletion is denied before any object is removed or tombstoned

r[molten.retention.operator_holds] Molten MUST support operator, legal, and compliance holds as explicit authority-bound retention pins.

#### Scenario: Legal hold blocks destructive action
- GIVEN an object under a legal hold retention class or pin source
- WHEN a destructive retention action is requested
- THEN the action is denied until the hold is cleared by explicit authority

r[molten.retention.cache_pins] Molten MUST distinguish remote sync cache pins and evaluation cache pins from durable pins.

#### Scenario: Cache pin is indexed separately
- GIVEN a remote cache or evaluation cache pin
- WHEN the reference index is built
- THEN the pin source remains visible in the index and decision receipt diagnostics

### Requirement: Auditable deletion, tombstone, redaction, and compaction
r[molten.retention.tombstones] Molten MUST represent deleted or redacted content with canonical tombstone records that expose audit metadata without leaking private content.

#### Scenario: Private redaction leaves public tombstone
- GIVEN a private secret ref eligible for redaction
- WHEN redaction is admitted
- THEN a tombstone records the object kind, class, action, policy refs, and evidence refs without plaintext

r[molten.retention.compaction] Molten MUST define compaction decisions as retention receipts that preserve audit semantics and deny compaction for classes where summaries would hide required evidence.

#### Scenario: Private secret compaction is denied
- GIVEN a private secret ref
- WHEN compaction is requested
- THEN compaction is denied unless policy explicitly provides a safe redaction path

r[molten.retention.secret_redaction_hook] Molten MUST coordinate private secret retention with redaction and reveal policy so secret refs are not physically removed without tombstone evidence.

#### Scenario: Secret redaction uses retention evidence
- GIVEN an encrypted ref from a private repro bundle
- WHEN it is redacted or tombstoned
- THEN the retention receipt binds policy and evidence refs for the redaction decision

r[molten.retention.remote_gc] Molten MUST consider remote replica and cache refs before local deletion without assuming global deletion authority.

#### Scenario: Remote refs block incomplete proof
- GIVEN remote cache refs that have not been reconciled
- WHEN GC eligibility is evaluated with incomplete proof
- THEN deletion is denied with a remote-cache diagnostic

### Requirement: Retention tests and invariants
r[molten.retention.pin_tests] Molten MUST test that pinned artifacts, blobs, receipts, and private refs cannot be deleted.

#### Scenario: Pinned object deletion test
- GIVEN an object with an active retention pin
- WHEN deletion is requested
- THEN the retention receipt denies deletion

r[molten.retention.eligibility_tests] Molten MUST test that objects become eligible only after pins and retained refs are cleared.

#### Scenario: Unpin then tombstone
- GIVEN a pinned object
- WHEN the pin is removed by authority and no retained refs remain
- THEN a tombstone action can pass

r[molten.retention.tombstone_tests] Molten MUST test that replay and audit summaries can explain tombstoned or redacted content.

#### Scenario: Tombstone summary avoids plaintext
- GIVEN a redacted private ref tombstone
- WHEN it is summarized
- THEN the summary explains the action without revealing private content

r[molten.retention.property_tests] Molten MUST test no-dangling-retained-ref and deny-on-incomplete-proof invariants within bounded generated cases.

#### Scenario: Generated retained refs deny deletion
- GIVEN generated finite retained-ref sets
- WHEN GC eligibility is evaluated
- THEN non-empty retained refs or incomplete proofs deny deletion


