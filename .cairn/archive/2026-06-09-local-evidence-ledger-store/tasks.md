# Tasks: local-evidence-ledger-store

- [x] [serial] r[molten.artifacts.local_evidence_ledger.content_table] Add immutable content table keyed by canonical Preserves hash.
- [x] [serial] r[molten.artifacts.local_evidence_ledger.indexes] Derive indexes by artifact kind, suite ref, report ref, bundle ref, receipt refs, signer refs, and status.
- [x] [serial] r[molten.artifacts.local_evidence_ledger.import_export] Add CLI import/export commands for report, bundle, unpack directory, and receipt files.
- [x] [serial] r[molten.artifacts.local_evidence_ledger.validation_receipts] Record validation/import/export receipts rather than mutating artifact status in place.
- [x] [parallel] r[molten.artifacts.local_evidence_ledger.retention_gc] Add retention pins, dependency reachability, and dry-run GC reporting.
- [x] [parallel] r[molten.artifacts.local_evidence_ledger.tests] Add tests for hash immutability, index rebuilds, corrupted storage bytes, missing dependencies, and GC pin preservation.
