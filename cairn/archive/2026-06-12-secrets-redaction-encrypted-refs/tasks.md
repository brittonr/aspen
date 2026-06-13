## Phase 1: Confidentiality records

- [x] [serial] r[molten.secrets_redaction_encrypted_refs.secret_ref] Define `secret-ref-v1`, confidential labels, encrypted refs, redaction markers, reveal/decrypt/redact/cleanup receipts.
- [x] [serial] r[molten.secrets_redaction_encrypted_refs.field_labels] Add field-label metadata for envelopes, traces, receipts, snapshots, storage, transcripts, catalogs, reports, and bundles.
- [x] [parallel] r[molten.secrets_redaction_encrypted_refs.ledger_catalog] Classify confidentiality artifacts in ledger/catalog views.
- [x] [parallel] r[molten.secrets_redaction_encrypted_refs.schema_constants] Export schema constants for secret/redaction/encrypted-ref records.

## Phase 2: Redacted rendering and catalog/MCP

- [x] [serial] r[molten.secrets_redaction_encrypted_refs.redacted_views] Apply redacted views by default to catalog, MCP, report show, transcript render, diagnostics, and repro summaries.
- [x] [serial] r[molten.secrets_redaction_encrypted_refs.transform_receipts] Emit redaction transform receipts bound to source refs, output refs, policy refs, and profile refs.
- [x] [parallel] r[molten.secrets_redaction_encrypted_refs.hidden_ref_policy] Ensure hidden refs are not expanded without reveal authority.
- [x] [parallel] r[molten.secrets_redaction_encrypted_refs.no_plaintext_logs] Add tests/lints preventing plaintext secret rendering by default.

## Phase 3: Reveal/decrypt effects

- [x] [serial] r[molten.secrets_redaction_encrypted_refs.reveal_gate] Gate reveal through authority, policy, resource, and effect-handle evidence.
- [x] [serial] r[molten.secrets_redaction_encrypted_refs.decrypt_effect] Add an admitted decrypt effect profile with reveal/decrypt receipts and commitment checks.
- [x] [parallel] r[molten.secrets_redaction_encrypted_refs.commitment_replay] Support replay by comparing commitments when plaintext reveal is denied.
- [x] [parallel] r[molten.secrets_redaction_encrypted_refs.revocation_cleanup] Coordinate secret revocation with authority cleanup, storage tombstones, and retention/GC.

## Phase 4: Repro bundle profiles and tests

- [x] [serial] r[molten.secrets_redaction_encrypted_refs.private_bundle_profile] Add encrypted/private repro bundle profile with validated encrypted refs and reveal receipts.
- [x] [serial] r[molten.secrets_redaction_encrypted_refs.gate_preserving_redaction] Permit gate-preserving redacted bundles only when transform receipts prove no semantic evidence was lost.
- [x] [parallel] r[molten.secrets_redaction_encrypted_refs.negative_tests] Test missed marker, malformed encrypted ref, unauthorized reveal, stale transform receipt, and redaction leak denial.
- [x] [parallel] r[molten.secrets_redaction_encrypted_refs.property_tests] Add Hegel properties for redaction stability, no-plaintext rendering, commitment replay, and reveal authority monotonicity.
