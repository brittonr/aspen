# Artifact Registry Delta: local evidence ledger store

### Requirement: Content is immutable by canonical hash
r[molten.artifacts.local_evidence_ledger.content_table] The local evidence ledger MUST store canonical artifact bytes immutably by their Preserves hash.

#### Scenario: Duplicate import is idempotent
- GIVEN an artifact already present in the ledger
- WHEN the same canonical bytes are imported again
- THEN the ledger returns the same content ref
- AND no duplicate content record is required

#### Scenario: Hash mismatch is rejected
- GIVEN bytes presented under a claimed content ref
- WHEN the bytes hash to a different ref
- THEN import fails closed

### Requirement: Indexes are derived and rebuildable
r[molten.artifacts.local_evidence_ledger.indexes] Ledger indexes MUST be derivable from stored canonical artifacts and validation receipts.

#### Scenario: Rebuild preserves query results
- GIVEN a ledger with reports, receipts, bundles, and failures
- WHEN indexes are dropped and rebuilt from content records
- THEN listing by report ref, suite ref, bundle ref, and receipt kind returns the same artifacts

### Requirement: Retention pins protect dependencies
r[molten.artifacts.local_evidence_ledger.retention_gc] GC MUST preserve every artifact reachable from a retained pin or retained receipt dependency.

#### Scenario: Pinned bundle preserves embedded report and receipts
- GIVEN a pinned sealed repro bundle
- WHEN GC runs
- THEN the bundle, embedded report, suite, gate receipt, redaction evidence, and verify receipts remain available

#### Scenario: Unpinned diagnostic failure can be collected
- GIVEN an unpinned failure artifact with no retained dependencies
- WHEN GC runs with policy allowing diagnostic cleanup
- THEN the failure artifact may be removed
- AND the GC receipt records the removed refs
