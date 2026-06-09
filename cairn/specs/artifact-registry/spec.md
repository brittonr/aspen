# Artifact Registry Specification

## Purpose

Defines Molten artifact registry and local evidence ledger requirements.

## Requirements

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

### Requirement: Ledger import and export preserve canonical evidence
r[molten.artifacts.local_evidence_ledger.import_export] The local evidence ledger MUST provide import and export operations for canonical report, bundle, unpack directory, and receipt artifacts without changing their content refs.

#### Scenario: Exported bytes match imported bytes
- GIVEN a canonical artifact imported into the ledger
- WHEN the artifact is exported back to a file
- THEN the exported file bytes hash to the same content ref
- AND the ledger records import and export evidence for the operation

### Requirement: Ledger validation appends receipts
r[molten.artifacts.local_evidence_ledger.validation_receipts] Ledger validation MUST append validation, import, export, pin, and GC receipts instead of mutating stored artifact bytes or overwriting prior status.

#### Scenario: Validation rule changes append new evidence
- GIVEN an artifact that already has a validation receipt
- WHEN validation is run again under a newer rule set
- THEN the ledger stores a new validation receipt
- AND the original artifact bytes and prior receipt remain available by content ref

### Requirement: Ledger behavior has regression coverage
r[molten.artifacts.local_evidence_ledger.tests] The local evidence ledger SHOULD have regression tests for immutability, rebuildable indexes, corrupted bytes, missing dependencies, and retained dependency preservation.

#### Scenario: Corrupted storage is detected
- GIVEN ledger storage whose bytes no longer match the recorded content ref
- WHEN indexes are rebuilt or the artifact is read
- THEN the corruption is reported as a validation failure
- AND retained dependencies are not silently dropped
