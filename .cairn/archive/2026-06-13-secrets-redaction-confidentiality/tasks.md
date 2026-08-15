## Phase 1: Secret and confidentiality model

- [x] [serial] r[molten.secrets.secret_ref_model] Define canonical secret refs with commitments, scope, allowed use, encryption refs, redaction label, expiry/revocation refs, reveal policy refs, and evidence refs.
- [x] [serial] r[molten.secrets.field_labels] Define confidential field labels for envelopes, traces, receipts, snapshots, storage records, transcripts, and catalog views.
- [x] [parallel] r[molten.secrets.no_plaintext_logs] Document and enforce that plaintext secrets are not logged or rendered by default.
- [x] [parallel] r[molten.secrets.reveal_receipts] Emit receipts for reveal, decrypt, redact, deny, and export decisions.

## Phase 2: Redaction and rendering

- [x] [serial] r[molten.secrets.redaction_markers] Define canonical redaction markers with reason, safe commitment, schema/path, policy refs, and receipt refs.
- [x] [serial] r[molten.secrets.catalog_redaction] Apply redacted views to catalog/MCP, docs, transcript output, diagnostics, and trace rendering.
- [x] [parallel] r[molten.secrets.snapshot_export] Classify and redact snapshots and replay logs before export.
- [x] [parallel] r[molten.secrets.transcript_policy] Require transcripts to declare whether they may record or reveal secret-bearing effects.

## Phase 3: Encryption and replay

- [x] [serial] r[molten.secrets.encrypted_refs] Define encrypted blob/storage record envelopes with ciphertext refs, encryption metadata, schema refs, policy refs, and commitments.
- [x] [serial] r[molten.secrets.decrypt_effect] Gate decryption through effect handlers with explicit authority and receipts.
- [x] [parallel] r[molten.secrets.replay_commitments] Support replay by comparing commitments when plaintext reveal is not admitted.
- [x] [parallel] r[molten.secrets.revocation_cleanup] Coordinate secret revocation with authority cleanup and retention/GC.

## Phase 4: Tests

- [x] [serial] r[molten.secrets.redaction_tests] Add tests that protected trace/catalog/transcript fields render as redaction markers without reveal authority.
- [x] [serial] r[molten.secrets.reveal_tests] Add tests that authorized reveal/decrypt emits receipts and preserves audit refs.
- [x] [parallel] r[molten.secrets.replay_tests] Add tests for replay with commitments-only secret comparisons.
- [x] [parallel] r[molten.secrets.property_tests] Add Hegel property tests for no-plaintext-rendering, redaction marker stability, and reveal authority monotonicity.
