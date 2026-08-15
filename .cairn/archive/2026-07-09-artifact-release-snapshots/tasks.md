## Tasks

- [x] [serial] r[molten.release_snapshots.namespace_snapshot_artifacts] Define immutable release/package snapshot artifacts that bind namespace scope, exact artifact refs, dependency closure digest, docs/transcripts, policy/provenance/source-gate/resource evidence, compatibility/migration receipts, caveats, redaction profile, and signatures.
- [x] [serial] r[molten.release_snapshots.closure_integrity] Verify snapshot closure integrity by recomputing artifact refs, dependency indexes, expected members, signatures, caveats, evidence freshness, and redaction profile.
- [x] [parallel] r[molten.release_snapshots.channel_view_non_authority] Model release channels as mutable name views pointing to immutable snapshot refs and deny channel-only trust or deployment authority.
- [x] [parallel] r[molten.release_snapshots.evidence_caveats] Require snapshot summaries and catalog views to surface caveats, pilot scope, stale evidence, redactions, and non-claim boundaries.
- [x] [serial] r[molten.release_snapshots.validation] Add positive and negative fixtures for snapshot creation, verification, channel update, tampered members, missing closure members, stale evidence, redaction, unauthorized channel moves, rollback, and channel-only trust denial.
