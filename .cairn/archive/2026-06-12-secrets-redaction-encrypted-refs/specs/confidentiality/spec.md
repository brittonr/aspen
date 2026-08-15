# Confidentiality Delta: Secrets, Redaction, and Encrypted Refs

### Requirement: Plaintext secrets are not rendered by default
r[molten.secrets_redaction_encrypted_refs.spec.no_plaintext_default] Secret-bearing values MUST render as canonical redaction markers unless the caller supplies passing reveal authority.

#### Scenario: Catalog redacts secret field
- GIVEN an artifact containing a field labeled confidential
- WHEN an unprivileged catalog view renders it
- THEN the field is replaced by a `redaction-marker-v1`
- AND the marker binds a safe commitment and redaction receipt

#### Scenario: Authorized reveal emits receipt
- GIVEN a caller with authority to reveal a secret for debugging
- WHEN reveal is requested
- THEN Molten emits a `reveal-receipt-v1`
- AND any plaintext ref is bound to that receipt and purpose

### Requirement: Confidentiality records are canonical
r[molten.secrets_redaction_encrypted_refs.secret_ref] Molten MUST define canonical `secret-ref-v1`, `confidential-label-v1`, `encrypted-ref-v1`, `redaction-marker-v1`, `reveal-receipt-v1`, `decrypt-receipt-v1`, `redaction-transform-receipt-v1`, and `secret-cleanup-receipt-v1` records with content-addressed refs and no plaintext-by-default rendering.

#### Scenario: Secret record binds commitment
- GIVEN secret material represented by a commitment
- WHEN a secret ref is emitted
- THEN the record binds scope, allowed uses, commitment, encryption profile, redaction label, revocation refs, evidence refs, and checks
- AND plaintext material is not embedded in the record

### Requirement: Confidential field labels are explicit
r[molten.secrets_redaction_encrypted_refs.field_labels] Molten MUST define confidential field label metadata for envelopes, traces, receipts, snapshots, storage, transcripts, catalogs, reports, and bundles.

#### Scenario: Field label covers report output
- GIVEN a report field that may contain credentials
- WHEN confidentiality metadata is emitted
- THEN the label identifies surface, canonical field path, classification, schema ref, policy refs, and no-plaintext-default checks

### Requirement: Confidentiality artifacts are classified
r[molten.secrets_redaction_encrypted_refs.ledger_catalog] Molten MUST classify confidentiality artifacts in local ledger, catalog, and MCP views.

#### Scenario: Ledger classifies reveal receipt
- GIVEN a canonical reveal receipt imported into the local ledger
- WHEN catalog or MCP views inspect the artifact
- THEN the artifact kind is `reveal-receipt`
- AND unprivileged views still redact secret-bearing payloads

### Requirement: Confidentiality schemas are exported
r[molten.secrets_redaction_encrypted_refs.schema_constants] Molten MUST export schema constants for secret, redaction, encrypted-ref, reveal/decrypt, cleanup, replay, and private bundle profile records.

#### Scenario: Schema constants are bound
- GIVEN a parser for a confidentiality artifact
- WHEN the record schema string is checked
- THEN it matches an exported Molten schema constant
- AND mismatched schema strings are rejected

### Requirement: Redacted views are default
r[molten.secrets_redaction_encrypted_refs.redacted_views] Molten MUST apply redacted views by default to catalog, MCP, report show, transcript render, diagnostics, and repro summaries unless reveal authority is supplied.

#### Scenario: Transcript output redacts secret
- GIVEN transcript output containing a secret marker
- WHEN the transcript is rendered without reveal authority
- THEN the output is replaced by a redaction marker
- AND plaintext is omitted from the rendered text

### Requirement: Redaction transforms are receipted
r[molten.secrets_redaction_encrypted_refs.transform_receipts] Molten MUST emit redaction transform receipts bound to source refs, output refs, policy refs, profile refs, marker refs, gate-preserving status, diagnostics, and checks.

#### Scenario: Transform receipt binds source and output
- GIVEN a secret-bearing value redacted for catalog view
- WHEN a redaction marker is emitted
- THEN the transform receipt binds the source ref, redacted output ref, policy/profile refs, and marker refs

### Requirement: Hidden refs are not expanded without reveal authority
r[molten.secrets_redaction_encrypted_refs.hidden_ref_policy] Molten MUST NOT expand hidden refs or secret-bearing content refs in catalog/MCP/report/repro views without passing reveal authority.

#### Scenario: Hidden encrypted ref remains hidden
- GIVEN a hidden encrypted ref in catalog visibility input
- WHEN MCP search or view is called without reveal authority
- THEN the response omits the hidden ref and exposes only safe commitments or markers

### Requirement: Plaintext logging is denied
r[molten.secrets_redaction_encrypted_refs.no_plaintext_logs] Molten MUST include tests or lint-backed checks preventing plaintext secret rendering by default.

#### Scenario: Secret marker is not rendered
- GIVEN a value containing a credential marker
- WHEN a default rendered view is produced
- THEN the output contains a redaction marker
- AND the credential text is absent

### Requirement: Encrypted refs are not authority
r[molten.secrets_redaction_encrypted_refs.spec.encrypted_ref_gate] Possession of an encrypted ref or ciphertext content ref MUST NOT grant decrypt or reveal authority.

#### Scenario: Ciphertext without authority denies
- GIVEN an encrypted ref present in a repro bundle
- AND no authority context admitting reveal/decrypt
- WHEN unpack or render attempts plaintext access
- THEN Molten emits a denial receipt
- AND only commitments/redaction markers are exposed

### Requirement: Reveal requests are authority gated
r[molten.secrets_redaction_encrypted_refs.reveal_gate] Molten MUST gate reveal through explicit authority, policy, resource, and effect-handle evidence and emit pass/deny reveal receipts.

#### Scenario: Missing authority denies reveal
- GIVEN a secret ref and a reveal request without authority evidence
- WHEN reveal is attempted
- THEN the reveal receipt decision is `deny`
- AND no plaintext ref is exposed

### Requirement: Decrypt effects are admitted explicitly
r[molten.secrets_redaction_encrypted_refs.decrypt_effect] Molten MUST add an admitted decrypt effect profile that requires reveal/decrypt receipts and commitment checks before plaintext refs are exposed.

#### Scenario: Encrypted ref alone cannot decrypt
- GIVEN an encrypted ref and matching commitment
- AND no passing reveal receipt
- WHEN decrypt is requested
- THEN the decrypt receipt decision is `deny`
- AND diagnostics state that encrypted refs alone are not authority

### Requirement: Replay can use commitments
r[molten.secrets_redaction_encrypted_refs.spec.commitment_replay] Deterministic replay MUST compare secret commitments when plaintext reveal is not admitted, and MUST require recorded effect responses or reveal receipts for operations needing plaintext.

#### Scenario: Commitment-only replay passes
- GIVEN a replay log whose secret-bearing observations include commitments
- AND plaintext reveal is denied
- WHEN replay compares canonical outputs
- THEN matching commitments are sufficient for pass evidence

#### Scenario: Plaintext-required effect denies without reveal
- GIVEN an effect requiring secret plaintext
- AND no recorded effect response or reveal receipt
- WHEN deterministic gate validation runs
- THEN the run is denied or classified diagnostic-only

### Requirement: Commitment replay receipts are canonical
r[molten.secrets_redaction_encrypted_refs.commitment_replay] Molten MUST emit commitment replay receipts that compare expected and actual commitments, bind reveal receipt refs when present, and deny plaintext-required replay without reveal or recorded effect evidence.

#### Scenario: Commitment replay mismatch denies
- GIVEN expected and actual secret commitments that differ
- WHEN commitment replay is validated
- THEN Molten emits a denial receipt
- AND diagnostics identify the commitment mismatch

### Requirement: Revocation cleanup is coordinated
r[molten.secrets_redaction_encrypted_refs.revocation_cleanup] Molten MUST coordinate secret revocation with authority cleanup, storage tombstones, retention refs, and GC evidence.

#### Scenario: Cleanup binds tombstone
- GIVEN a revoked secret ref
- WHEN cleanup is admitted
- THEN the cleanup receipt binds revocation, tombstone, retention refs, authority refs, and policy refs

### Requirement: Private repro bundle profile exists
r[molten.secrets_redaction_encrypted_refs.private_bundle_profile] Molten MUST define an encrypted/private repro bundle profile with validated encrypted refs, reveal receipts, transform receipts, and fail-closed parsing.

#### Scenario: Private profile binds encrypted refs
- GIVEN a private repro bundle profile
- WHEN it is parsed
- THEN encrypted refs, reveal receipt refs, transform receipt ref, and profile ref are validated

### Requirement: Gate-preserving redaction is explicit
r[molten.secrets_redaction_encrypted_refs.gate_preserving_redaction] Molten MUST permit gate-preserving redacted bundles only when transform receipts prove no semantic evidence needed for the gate was lost.

#### Scenario: Gate preserving transform passes
- GIVEN a transform receipt marked gate-preserving
- WHEN the private bundle profile is accepted
- THEN checks include semantic evidence preservation and transform binding

### Requirement: Confidentiality negative tests exist
r[molten.secrets_redaction_encrypted_refs.negative_tests] Molten MUST test missed markers, malformed encrypted refs, unauthorized reveal, stale transform receipt, and redaction leak denial.

#### Scenario: Unauthorized reveal test denies
- GIVEN a reveal request without authority refs
- WHEN the confidentiality negative suite runs
- THEN the reveal receipt is `deny`
- AND plaintext refs are omitted

### Requirement: Confidentiality properties hold
r[molten.secrets_redaction_encrypted_refs.property_tests] Molten MUST include property tests for redaction stability, no-plaintext rendering, commitment replay, and reveal authority monotonicity.

#### Scenario: Redaction is stable
- GIVEN the same secret-bearing value generated twice
- WHEN it is redacted with the same profile
- THEN the redacted marker ref is stable
- AND the rendered value contains no plaintext
