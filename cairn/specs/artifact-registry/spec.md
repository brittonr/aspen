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

### Requirement: Chain links are immutable ledger artifacts
r[molten.evidence.chain_hashing.ledger_index] The local evidence ledger MUST store chain links as immutable canonical artifacts and derive chain indexes from stored link bytes and linked payload artifacts.

#### Scenario: Rebuilding indexes preserves chain heads
r[molten.evidence.chain_hashing.ledger_index.rebuild]
- GIVEN a ledger containing chain links, payload artifacts, append receipts, and checkpoints
- WHEN derived indexes are dropped and rebuilt from canonical content
- THEN chain scope/id/epoch listings, parent/child relationships, sequence lookups, payload lookups, anchors, checkpoints, and heads are reconstructed

#### Scenario: Indexed head is not authoritative without link bytes
r[molten.evidence.chain_hashing.ledger_index.head_requires_content]
- GIVEN a derived index entry claiming a chain head
- WHEN the corresponding canonical chain-link artifact is missing or hashes differently
- THEN the ledger rejects the head until the canonical link bytes are available and verified

### Requirement: Append receipts record head transitions
r[molten.evidence.chain_hashing.append_receipts] Ledger chain appends MUST emit canonical append receipts that bind head-before, head-after, appended link ref, payload ref, and continuity checks.

#### Scenario: Idempotent append of existing head
r[molten.evidence.chain_hashing.append_receipts.idempotent]
- GIVEN a chain head already points to a link ref
- WHEN the same canonical link is appended again with the same head-before and head-after
- THEN append is idempotent
- AND the append receipt names the existing link ref

#### Scenario: Unexpected stale head is denied
r[molten.evidence.chain_hashing.append_receipts.stale]
- GIVEN a chain head has advanced since a caller last observed it
- WHEN the caller appends a link against the stale head without an admitted fork or historical policy
- THEN append fails closed
- AND a denial receipt names the stale observed head and current head

### Requirement: Checkpoints are explicit artifacts
r[molten.evidence.chain_hashing.control_plane_checkpoints] Accepted control-plane chain heads SHOULD be represented by canonical checkpoint artifacts or receipts that name chain scope/id/epoch, prior checkpoint ref, new head ref, verified range, and policy/membership refs.

#### Scenario: Checkpoint descends from prior checkpoint
r[molten.evidence.chain_hashing.control_plane_checkpoints.descends]
- GIVEN a prior accepted checkpoint for a chain scope/id/epoch
- WHEN a new checkpoint is proposed
- THEN verification confirms the new head descends from the prior checkpoint head or explicitly records an admitted reconfiguration/epoch change

### Requirement: GC preserves anchored chains
r[molten.evidence.chain_hashing.anchor_policy] Retention and GC MUST preserve chain links and payload artifacts reachable from retained anchors, heads, checkpoints, or signed append/verify receipts.

#### Scenario: Pinned checkpoint preserves segment
r[molten.evidence.chain_hashing.anchor_policy.gc]
- GIVEN a retained checkpoint naming a chain head
- WHEN GC runs
- THEN the checkpoint, verified segment to the retained anchor, append/verify receipts, and required payload artifacts remain available
