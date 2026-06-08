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

### Requirement: Retention-gated destructive paths
r[molten.retention.ledger_gc_gate] Molten MUST gate evidence-ledger garbage collection through passing retention receipts before removing ledger content.

#### Scenario: Ledger GC denied before removal
- GIVEN a ledger artifact with an active retention pin
- WHEN ledger GC evaluates the artifact for removal
- THEN GC emits denial evidence and does not remove the artifact

r[molten.retention.chunk_gc_gate] Molten MUST gate chunk-store manifest and chunk removal through passing retention receipts before removing files or writing tombstone receipts.

#### Scenario: Chunk GC denied before tombstone
- GIVEN an unpinned chunk-store manifest that is still retention-pinned
- WHEN chunk GC evaluates the manifest for removal
- THEN no manifest or chunk is removed and the GC receipt binds the denying retention receipt

r[molten.retention.eval_cache_tombstone_gate] Molten MUST gate evaluation-cache invalidation tombstones through passing retention receipts before writing tombstone entries.

#### Scenario: Cache tombstone denied before mutation
- GIVEN an evaluation-cache key with active retention evidence
- WHEN invalidation selects the key
- THEN the tombstone is not written unless retention eligibility passes

r[molten.retention.secret_cleanup_gate] Molten MUST require secret cleanup receipts to bind actual passing retention receipts for the cleaned secret and tombstone.

#### Scenario: Secret cleanup rejects stale retention evidence
- GIVEN a secret cleanup request with missing or mismatched retention evidence
- WHEN the cleanup receipt is built
- THEN cleanup is denied and diagnostics identify the retention mismatch

### Requirement: Subsystem retention evidence
r[molten.retention.subsystem_receipt_refs] Molten MUST expose retention receipt refs in ledger GC, chunk GC, cache invalidation, and secret cleanup receipts without treating them as authority grants.

#### Scenario: Subsystem receipt binds retention refs
- GIVEN a destructive subsystem decision that evaluated retention
- WHEN the subsystem receipt is emitted
- THEN the receipt lists the retention receipt refs that informed the decision

r[molten.retention.destructive_gate_tests] Molten MUST test pass and fail-closed retention-gated destructive paths for ledger GC, chunk GC, cache tombstones, and secret cleanup.

#### Scenario: Denials leave content intact
- GIVEN bounded generated or fixture destructive candidates with incomplete or denied retention decisions
- WHEN the subsystem attempts cleanup
- THEN tests verify content remains intact and denial receipts are auditable

### Requirement: Explicit destructive retention evidence
r[molten.retention.destructive_evidence_inputs] Molten MUST require destructive subsystem retention evaluations to accept explicit requester, policy, authority, evidence, retained-reference, remote-reference, and reference-index completeness inputs rather than relying on mutable names or synthesized trust.

#### Scenario: Destructive caller supplies evidence inputs
- GIVEN a ledger GC, chunk GC, or cache invalidation candidate
- WHEN the subsystem evaluates retention eligibility
- THEN the retention evaluation binds the explicit requester, policy refs, evidence refs, retained refs, remote refs, and reference-index completeness supplied by the caller

r[molten.retention.apply_requires_authority] Molten MUST deny apply-mode destructive candidates when requester, policy, authority, or supporting evidence refs are missing.

#### Scenario: Missing authority denies before removal
- GIVEN an apply-mode destructive candidate without delete authority evidence
- WHEN the subsystem attempts removal or tombstoning
- THEN the operation emits denial evidence and does not remove or tombstone the object

r[molten.retention.reference_index_plumbing] Molten MUST pass retained refs, remote refs, and reference-index completeness through destructive subsystem retention checks so incomplete proofs fail closed.

#### Scenario: Remote uncertainty blocks apply
- GIVEN a destructive candidate with unresolved remote cache refs or an incomplete reference index
- WHEN apply-mode GC or invalidation evaluates the candidate
- THEN deletion or tombstoning is denied before mutation

### Requirement: Destructive retention evidence receipts
r[molten.retention.cli_evidence_flags] Molten MUST expose operator-facing CLI flags for destructive retention requester, policy, authority, evidence, retained, remote, and reference-index completeness inputs.

#### Scenario: CLI surfaces missing evidence
- GIVEN a destructive CLI command without required retention evidence flags
- WHEN candidates are selected for apply-mode mutation
- THEN the command emits a denial receipt and reports the missing evidence diagnostics

r[molten.retention.evidence_summary_receipts] Molten MUST bind retention evidence summaries in subsystem GC and invalidation receipts without treating those summaries as authority grants.

#### Scenario: Receipt records evidence summary
- GIVEN a destructive subsystem decision
- WHEN the subsystem receipt is emitted
- THEN it records the retention receipt refs and the retention evidence inputs that informed the decision

r[molten.retention.destructive_evidence_tests] Molten MUST test fail-closed destructive retention evidence behavior for missing authority, missing policy, missing evidence, incomplete indexes, retained refs, and remote uncertainty.

#### Scenario: Evidence tests leave content intact
- GIVEN destructive candidates with incomplete or missing retention evidence
- WHEN subsystem cleanup runs
- THEN tests verify denial receipts are auditable and selected content remains intact
