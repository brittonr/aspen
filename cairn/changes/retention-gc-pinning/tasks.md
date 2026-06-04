## Phase 1: Pin and retention model

- [ ] [serial] r[molten.retention.pin_model] Define canonical pin records with object ref, source, reason, owner, expiry, policy refs, and evidence refs.
- [ ] [serial] r[molten.retention.classes] Define retention classes for ephemeral cache, debug trace, replay snapshot, audit receipt, durable value, public artifact, private secret ref, upgrade rollback, and legal hold.
- [ ] [parallel] r[molten.retention.no_name_gc] Document that mutable names/aliases are insufficient for deletion eligibility.
- [ ] [parallel] r[molten.retention.receipts] Emit receipts for pin, unpin, retain, eligibility, deletion, tombstone, redaction, compaction, and denial.

## Phase 2: Reference indexes

- [ ] [serial] r[molten.retention.reference_index] Track references from active sessions, artifacts, blobs, receipts, snapshots, transcripts, docs, policies, upgrades, and storage refs.
- [ ] [serial] r[molten.retention.gc_eligibility] Deny GC unless reference indexes prove no active or retained dependency remains.
- [ ] [parallel] r[molten.retention.operator_holds] Support operator/legal/compliance hold pins with explicit authority.
- [ ] [parallel] r[molten.retention.cache_pins] Track remote sync and evaluation cache pins separately from durable pins.

## Phase 3: Deletion behavior

- [ ] [serial] r[molten.retention.tombstones] Represent tombstones/redaction markers for deleted or redacted content.
- [ ] [serial] r[molten.retention.compaction] Define trace/receipt compaction rules that preserve audit semantics when admitted.
- [ ] [parallel] r[molten.retention.secret_redaction_hook] Coordinate private/secret retention with redaction/confidentiality policy.
- [ ] [parallel] r[molten.retention.remote_gc] Plan remote replica/cache deletion signaling without assuming global deletion authority.

## Phase 4: Tests

- [ ] [serial] r[molten.retention.pin_tests] Add tests that pinned artifacts/blobs/receipts cannot be deleted.
- [ ] [serial] r[molten.retention.eligibility_tests] Add tests for deletion after all pins and retained refs are removed.
- [ ] [parallel] r[molten.retention.tombstone_tests] Add tests that replay/audit can explain tombstoned or redacted content.
- [ ] [parallel] r[molten.retention.property_tests] Add Hegel property tests for no-dangling-retained-ref and deny-on-incomplete-proof invariants.
