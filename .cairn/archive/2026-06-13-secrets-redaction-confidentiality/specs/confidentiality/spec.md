# Confidentiality Delta: Secrets, Redaction, and Encrypted Refs

## Requirements

### Requirement: Secret refs and encrypted refs MUST be canonical and non-authorizing
r[molten.secrets.secret_ref_model] Molten MUST define canonical secret refs with commitments, scope, allowed uses, encryption refs, redaction labels, expiry/revocation refs, reveal policy refs, and evidence refs.
r[molten.secrets.encrypted_refs] Molten MUST define encrypted blob/storage record envelopes with ciphertext refs, encryption metadata, schema refs, policy refs, and commitments.
r[molten.secrets_redaction_encrypted_refs.secret_ref] Molten MUST define canonical `secret-ref-v1`, `confidential-label-v1`, `encrypted-ref-v1`, `redaction-marker-v1`, `reveal-receipt-v1`, `decrypt-receipt-v1`, `redaction-transform-receipt-v1`, and `secret-cleanup-receipt-v1` records with content-addressed refs and no plaintext-by-default rendering.
r[molten.secrets_redaction_encrypted_refs.spec.encrypted_ref_gate] Possession of an encrypted ref or ciphertext content ref MUST NOT grant decrypt or reveal authority.

#### Scenario: Secret record binds commitment without plaintext
- GIVEN secret material represented by a commitment
- WHEN a secret ref is emitted
- THEN the record binds scope, allowed uses, commitment, encryption profile, redaction label, revocation refs, evidence refs, and checks without embedding plaintext material.

#### Scenario: Ciphertext alone denies reveal
- GIVEN an encrypted ref and no authority context admitting reveal or decrypt
- WHEN plaintext access is attempted
- THEN Molten emits a denial receipt and exposes only commitments or redaction markers.

### Requirement: Confidential field labels MUST cover secret-bearing surfaces
r[molten.secrets.field_labels] Molten MUST define confidential field labels for envelopes, traces, receipts, snapshots, storage records, transcripts, catalog views, reports, and bundles.
r[molten.secrets_redaction_encrypted_refs.field_labels] Molten MUST define confidential field label metadata for envelopes, traces, receipts, snapshots, storage, transcripts, catalogs, reports, and bundles.
r[molten.secrets_redaction_encrypted_refs.schema_constants] Molten MUST export schema constants for secret, redaction, encrypted-ref, reveal/decrypt, cleanup, replay, and private bundle profile records.

#### Scenario: Field label covers report output
- GIVEN a report field that may contain credentials
- WHEN confidentiality metadata is emitted
- THEN the label identifies surface, canonical field path, classification, schema ref, policy refs, and no-plaintext-default checks.

### Requirement: Plaintext secrets MUST NOT render by default
r[molten.secrets.no_plaintext_logs] Molten MUST document and enforce that plaintext secrets are not logged or rendered by default.
r[molten.secrets_redaction_encrypted_refs.spec.no_plaintext_default] Secret-bearing values MUST render as canonical redaction markers unless the caller supplies passing reveal authority.
r[molten.secrets_redaction_encrypted_refs.no_plaintext_logs] Molten MUST include tests or lint-backed checks preventing plaintext secret rendering by default.

#### Scenario: Secret marker is not rendered
- GIVEN a value containing a credential marker
- WHEN a default rendered view is produced
- THEN the output contains a redaction marker and omits the credential text.

### Requirement: Reveal and decrypt decisions MUST be receipted and authority gated
r[molten.secrets.reveal_receipts] Molten MUST emit receipts for reveal, decrypt, redact, deny, and export decisions.
r[molten.secrets.decrypt_effect] Molten MUST gate decryption through effect handlers with explicit authority and receipts.
r[molten.secrets_redaction_encrypted_refs.reveal_gate] Molten MUST gate reveal through explicit authority, policy, resource, and effect-handle evidence and emit pass/deny reveal receipts.
r[molten.secrets_redaction_encrypted_refs.decrypt_effect] Molten MUST add an admitted decrypt effect profile that requires reveal/decrypt receipts and commitment checks before plaintext refs are exposed.

#### Scenario: Missing authority denies reveal
- GIVEN a secret ref and a reveal request without authority evidence
- WHEN reveal is attempted
- THEN the reveal receipt decision is `deny` and no plaintext ref is exposed.

#### Scenario: Encrypted ref alone cannot decrypt
- GIVEN an encrypted ref and matching commitment but no passing reveal receipt
- WHEN decrypt is requested
- THEN the decrypt receipt decision is `deny` and diagnostics state that encrypted refs alone are not authority.

### Requirement: Redaction markers and transforms MUST preserve audit structure
r[molten.secrets.redaction_markers] Molten MUST define canonical redaction markers with reason, safe commitment, schema/path, policy refs, and receipt refs.
r[molten.secrets_redaction_encrypted_refs.transform_receipts] Molten MUST emit redaction transform receipts bound to source refs, output refs, policy refs, profile refs, marker refs, gate-preserving status, diagnostics, and checks.
r[molten.secrets_redaction_encrypted_refs.gate_preserving_redaction] Molten MUST permit gate-preserving redacted bundles only when transform receipts prove no semantic evidence needed for the gate was lost.

#### Scenario: Transform receipt binds source and output
- GIVEN a secret-bearing value redacted for a catalog or repro view
- WHEN a redaction marker is emitted
- THEN the transform receipt binds the source ref, redacted output ref, policy/profile refs, marker refs, and gate-preserving status.

### Requirement: Redacted views MUST be default for catalog, MCP, diagnostics, transcript, and repro outputs
r[molten.secrets.catalog_redaction] Molten MUST apply redacted views to catalog/MCP, docs, transcript output, diagnostics, and trace rendering.
r[molten.secrets_redaction_encrypted_refs.redacted_views] Molten MUST apply redacted views by default to catalog, MCP, report show, transcript render, diagnostics, and repro summaries unless reveal authority is supplied.
r[molten.secrets_redaction_encrypted_refs.hidden_ref_policy] Molten MUST NOT expand hidden refs or secret-bearing content refs in catalog/MCP/report/repro views without passing reveal authority.
r[molten.secrets_redaction_encrypted_refs.ledger_catalog] Molten MUST classify confidentiality artifacts in local ledger, catalog, and MCP views.

#### Scenario: Catalog redacts secret field
- GIVEN an artifact containing a field labeled confidential
- WHEN an unprivileged catalog or MCP view renders it
- THEN the field is replaced by a `redaction-marker-v1` that binds a safe commitment and redaction receipt.

#### Scenario: Hidden encrypted ref remains hidden
- GIVEN a hidden encrypted ref in catalog visibility input
- WHEN MCP search or view is called without reveal authority
- THEN the response omits the hidden ref and exposes only safe commitments or markers.

### Requirement: Snapshot, replay, and transcript exports MUST declare and enforce secret handling
r[molten.secrets.snapshot_export] Molten MUST classify and redact snapshots and replay logs before export.
r[molten.secrets.transcript_policy] Molten MUST require transcripts to declare whether they may record or reveal secret-bearing effects.
r[molten.secrets.replay_commitments] Molten MUST support replay by comparing commitments when plaintext reveal is not admitted.
r[molten.secrets_redaction_encrypted_refs.spec.commitment_replay] Deterministic replay MUST compare secret commitments when plaintext reveal is not admitted, and MUST require recorded effect responses or reveal receipts for operations needing plaintext.
r[molten.secrets_redaction_encrypted_refs.commitment_replay] Molten MUST emit commitment replay receipts that compare expected and actual commitments, bind reveal receipt refs when present, and deny plaintext-required replay without reveal or recorded effect evidence.

#### Scenario: Commitment-only replay passes
- GIVEN a replay log whose secret-bearing observations include commitments and plaintext reveal is denied
- WHEN replay compares canonical outputs
- THEN matching commitments are sufficient for pass evidence.

#### Scenario: Transcript requires declaration
- GIVEN a transcript runner that records effect output
- WHEN an effect may contain secret-bearing data
- THEN the transcript policy declares whether plaintext, encrypted payloads, or commitments are recorded before export.

### Requirement: Secret revocation cleanup MUST coordinate with authority and retention evidence
r[molten.secrets.revocation_cleanup] Molten MUST coordinate secret revocation with authority cleanup and retention/GC.
r[molten.secrets_redaction_encrypted_refs.revocation_cleanup] Molten MUST coordinate secret revocation with authority cleanup, storage tombstones, retention refs, and GC evidence.

#### Scenario: Cleanup binds tombstone
- GIVEN a revoked secret ref
- WHEN cleanup is admitted
- THEN the cleanup receipt binds revocation, tombstone, retention refs, authority refs, and policy refs.

### Requirement: Private repro bundle profiles MUST remain fail-closed
r[molten.secrets_redaction_encrypted_refs.private_bundle_profile] Molten MUST define an encrypted/private repro bundle profile with validated encrypted refs, reveal receipts, transform receipts, and fail-closed parsing.

#### Scenario: Private profile binds encrypted refs
- GIVEN a private repro bundle profile
- WHEN it is parsed
- THEN encrypted refs, reveal receipt refs, transform receipt ref, and profile ref are validated.

### Requirement: Confidentiality tests MUST cover redaction, reveal, replay, and properties
r[molten.secrets.redaction_tests] Molten MUST add tests that protected trace/catalog/transcript fields render as redaction markers without reveal authority.
r[molten.secrets.reveal_tests] Molten MUST add tests that authorized reveal/decrypt emits receipts and preserves audit refs.
r[molten.secrets.replay_tests] Molten MUST add tests for replay with commitments-only secret comparisons.
r[molten.secrets.property_tests] Molten MUST add Hegel property tests for no-plaintext-rendering, redaction marker stability, and reveal authority monotonicity.
r[molten.secrets_redaction_encrypted_refs.negative_tests] Molten MUST test missed markers, malformed encrypted refs, unauthorized reveal, stale transform receipt, and redaction leak denial.
r[molten.secrets_redaction_encrypted_refs.property_tests] Molten MUST include property tests for redaction stability, no-plaintext rendering, commitment replay, and reveal authority monotonicity.

#### Scenario: Unauthorized reveal test denies
- GIVEN a reveal request without authority refs
- WHEN the confidentiality negative suite runs
- THEN the reveal receipt is `deny` and plaintext refs are omitted.

#### Scenario: Redaction is stable
- GIVEN the same secret-bearing value generated twice
- WHEN it is redacted with the same profile
- THEN the redacted marker ref is stable and the rendered value contains no plaintext.
