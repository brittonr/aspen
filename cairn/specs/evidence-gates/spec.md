# Evidence Gates

## Purpose

Evidence gates define how Molten receipts are attributed, verified, chained, and kept separate from subsystem authority.

## Requirements

### Requirement: Signed envelopes preserve subject hashes
r[molten.evidence.signed_receipts.envelope] A signed receipt envelope MUST wrap a canonical receipt without changing the subject receipt's canonical ref.

#### Scenario: Signing a gate receipt preserves gate receipt ref
- GIVEN a canonical `<gate-receipt-v1 ...>`
- WHEN the receipt is signed
- THEN the signed envelope names the same gate receipt ref
- AND the signature covers the canonical bytes of that receipt

### Requirement: Signature verification is fail-closed
r[molten.evidence.signed_receipts.verify] Evidence profiles that require signatures MUST reject missing, malformed, unauthorized, or stale signatures before accepting pass evidence.

#### Scenario: Wrong key purpose is rejected
- GIVEN a signed receipt using a key authorized only for diagnostics
- WHEN a production gate profile requires pass-evidence signing
- THEN the receipt is rejected

#### Scenario: Mismatched subject ref is rejected
- GIVEN a signed envelope whose subject ref does not hash to the embedded receipt
- WHEN verification runs
- THEN verification fails closed

### Requirement: Local signer fixtures
r[molten.evidence.signed_receipts.key_fixtures] Molten SHOULD provide deterministic local signer, key, purpose, and trust-root fixtures for harness and CLI verification without treating those fixtures as production trust.

#### Scenario: Local fixture signs pass evidence
- GIVEN a local development receipt and fixture signer settings
- WHEN signing runs in the harness or CLI
- THEN the signed envelope verifies under the matching fixture purpose, trust root, and key
- AND production profiles MAY still require non-fixture trust roots

### Requirement: Generic signed receipt CLI
r[molten.evidence.signed_receipts.cli] Molten MUST provide CLI commands to sign canonical receipts and verify signed receipt envelopes with configured purpose, trust root, key, signer, and subject constraints.

#### Scenario: CLI verifies configured signature policy
- GIVEN a signed receipt envelope
- WHEN the CLI verifies it with an expected signer, subject ref, purpose, trust root, and key
- THEN verification passes only when all configured signature policy fields match

### Requirement: Signed receipt test coverage
r[molten.evidence.signed_receipts.tests] Molten SHOULD cover wrong signer, wrong purpose, wrong key, mismatched subject ref, malformed envelope, unsupported algorithm, and missing trust-root failures in automated tests.

#### Scenario: Signature negative tests fail closed
- GIVEN malformed or policy-mismatched signed receipt envelopes
- WHEN verification runs
- THEN tests assert verification fails closed with diagnostics before the receipt is accepted

### Requirement: Receipt chains are explicit
r[molten.evidence.signed_receipts.chain] Signed receipt envelopes SHOULD name parent receipt refs when one receipt depends on another.

#### Scenario: Verify receipt chains to report gate receipt
- GIVEN a sealed repro verify receipt
- WHEN it is signed
- THEN the signed envelope names the embedded report gate receipt as a parent
- AND chain verification can reconstruct the gate-to-verify dependency

### Requirement: Operator receipt signing CLI
r[molten.evidence.signed_receipts.operator_receipts_cli] Molten MUST expose top-level receipt signing and signed receipt verification commands for canonical dogfood and release evidence artifacts.

#### Scenario: Operator signs and verifies a release receipt
- GIVEN a canonical dogfood or release receipt file
- WHEN an operator runs receipt signing and signed verification with a signer, purpose, trust root, key, and optional expected subject ref
- THEN Molten emits or verifies a `signed-receipt-v1` envelope that binds the subject ref, signer identity, purpose, trust root, parent refs, and canonical subject bytes

### Requirement: Signed release bundle members
r[molten.evidence.signed_receipts.release_bundle_members] Release evidence bundle verification MAY require signed Preserves member receipts and MUST deny the bundle review when required signatures are missing, malformed, signed by the wrong signer, scoped to the wrong purpose, or bound to a subject ref outside the bundle.

#### Scenario: Bundle verification requires signed member receipts
- GIVEN a release evidence bundle with dogfood report, release gate, Nix evidence, and Nix verify Preserves members
- WHEN verification is run with signed members required
- THEN every Preserves member ref has a verified signed envelope for the configured signer, purpose, trust root, and key before the bundle receipt can pass

#### Scenario: Wrong signer denies bundle review
- GIVEN a signed member envelope from a signer that is not configured for the release review profile
- WHEN release bundle verification requires signed member receipts
- THEN it emits a `release-evidence-bundle-verify-receipt-v1` with decision `deny` and diagnostics identifying the signer mismatch

### Requirement: Signed receipt keyring records
r[molten.evidence.signed_receipts.keyring_records] Molten MUST represent signed receipt verification keys and key revocations as canonical evidence artifacts with stable refs, signer identity, trust root, key id, generation or revocation reason, and evidence-only checks.

#### Scenario: Key import writes auditable evidence
- GIVEN a signer id, trust root, key id, and local fixture verification key
- WHEN an operator imports the key into the signed receipt keyring
- THEN Molten stores a canonical `signed-receipt-key-v1` artifact in the ledger
- AND the artifact binds the signer, trust root, key id, generation, predecessor ref, and evidence-only caveat

#### Scenario: Revocation writes immutable evidence
- GIVEN an imported signed receipt key
- WHEN an operator revokes the key
- THEN Molten stores a canonical `signed-receipt-key-revocation-v1` artifact naming the revoked key ref
- AND future keyring verification treats that key as ineligible without mutating the original key record

### Requirement: Signed receipt keyring CLI
r[molten.evidence.signed_receipts.keyring_cli] Molten MUST expose ledger-backed CLI commands to import, list, show, revoke, and rotate signed receipt verification keys.

#### Scenario: Operator rotates a signing key
- GIVEN a current signed receipt key record
- WHEN an operator rotates it with a new key id and key material
- THEN Molten stores a new key record with a predecessor ref
- AND Molten stores a revocation record for the old key that names the new key as its successor

### Requirement: Signed receipt keyring verification
r[molten.evidence.signed_receipts.keyring_verify] Signed receipt verification MAY resolve keys from a ledger keyring and MUST fail closed when the selected key is missing, ambiguous, stale, revoked, scoped to the wrong signer, scoped to the wrong trust root, or unable to verify the envelope signature.

#### Scenario: Revoked key denies signed receipt verification
- GIVEN a signed receipt envelope that verifies with an imported key
- AND a key revocation record for that key is present in the keyring ledger
- WHEN verification runs with that keyring
- THEN verification fails closed with diagnostics that identify the revocation

#### Scenario: Ambiguous keyring denies verification
- GIVEN multiple current unrevoked key records for the same signer and trust root
- WHEN verification runs without a key id or key ref disambiguator
- THEN verification fails closed and requires an explicit key id or key ref

### Requirement: Release bundle signed members use keyring policy
r[molten.evidence.signed_receipts.keyring_release_bundle] Release evidence bundle verification MAY require signed member receipts to verify through a ledger keyring and MUST deny the bundle review when a required member is signed by a missing, ambiguous, stale, revoked, wrong-signer, wrong-purpose, or wrong-trust-root key.

#### Scenario: Bundle verification uses current keyring key
- GIVEN a release evidence bundle with signed Preserves members
- AND a keyring ledger containing the current unrevoked signer key
- WHEN release bundle verification runs with `--require-signed-members` and keyring inputs
- THEN every signed member must verify through the current key before the bundle verify receipt can pass

### Requirement: Signed receipt keyring remains evidence only
r[molten.evidence.signed_receipts.keyring_evidence_only] Signed receipt key records, revocation records, and keyring verification decisions MUST NOT grant authority, policy, provenance, resource, transport, source-gate, retention, destructive-operation trust, or permission to bypass subsystem gates.

#### Scenario: Current key does not grant release authority
- GIVEN a current unrevoked keyring key and passing signed receipt verification
- WHEN a subsystem performs privileged, destructive, transport, provenance-sensitive, source-gated, or retention-sensitive work
- THEN it still requires its own matching gate receipts and MUST NOT treat the keyring record or signed envelope as subsystem authority

### Requirement: Release promotion gate receipt
r[molten.evidence.release_promotion.receipt] Molten MUST emit a canonical `release-promotion-gate-receipt-v1` that aggregates release bundle verification, signed keyring currentness, source evidence, Octet evidence, Cairn evidence, diagnostics, and evidence-only caveats.

#### Scenario: Promotion receipt passes for complete evidence graph
- GIVEN a passing release evidence bundle verification receipt
- AND a current unrevoked signed receipt key selected from the keyring
- AND source, Octet, and Cairn evidence markers
- WHEN release promotion verification runs
- THEN Molten emits a `release-promotion-gate-receipt-v1` with decision `pass`
- AND the receipt binds the bundle verify ref, selected key ref, source evidence ref, Octet evidence ref, and Cairn evidence ref

### Requirement: Release promotion CLI
r[molten.evidence.release_promotion.cli] Molten MUST expose `molten dogfood release-promote` to create release promotion receipts from a realized dogfood output, bundle verification receipt, signed keyring ledger, and explicit source/Octet/Cairn evidence markers.

#### Scenario: CLI writes pass or deny receipt
- GIVEN promotion inputs that are complete or incomplete
- WHEN the CLI runs
- THEN it writes a canonical promotion receipt to the requested output path
- AND denial diagnostics are stored in the receipt rather than only in logs

### Requirement: Promotion binds signed keyring currentness
r[molten.evidence.release_promotion.keyring] Release promotion MUST fail closed when the selected signed receipt key is missing, ambiguous, stale, revoked, scoped to the wrong signer, or scoped to the wrong trust root.

#### Scenario: Revoked key denies promotion
- GIVEN a release bundle verification receipt that otherwise passes
- AND a keyring ledger containing a revocation record for the selected signed receipt key
- WHEN release promotion runs with that key selected
- THEN the promotion receipt decision is `deny`
- AND diagnostics identify the signed keyring currentness failure

### Requirement: Promotion binds bundle verification and output path
r[molten.evidence.release_promotion.bundle] Release promotion MUST bind the release bundle verification receipt and the realized output path ref and MUST deny promotion when the bundle verification receipt is not passing or was produced for a different output path.

#### Scenario: Stale bundle verification denies promotion
- GIVEN a bundle verification receipt for one output path
- WHEN promotion is run against a different realized output path
- THEN the promotion receipt decision is `deny`
- AND diagnostics identify the output path ref mismatch

### Requirement: Promotion binds source, Octet, and Cairn evidence markers
r[molten.evidence.release_promotion.source_gates] Release promotion MUST bind explicit source, Octet, and Cairn evidence markers as deterministic refs and MUST deny promotion when any required marker is missing.

#### Scenario: Missing source gate marker denies promotion
- GIVEN a passing bundle verification receipt and current signed key
- WHEN the source evidence marker is empty
- THEN the promotion receipt decision is `deny`
- AND diagnostics identify the missing source evidence marker

### Requirement: Signed release promotion receipt
r[molten.evidence.release_promotion.signed_receipt] Molten's dogfood release evidence MUST sign the final `release-promotion-gate-receipt-v1` with a distinct `release-promotion` signed receipt purpose.

#### Scenario: Promotion receipt is signed after promotion passes
- GIVEN a dogfood release output with a passing release promotion gate receipt
- WHEN the dogfood release check finishes
- THEN it emits a signed receipt envelope for `release-promotion-gate.preserves`
- AND the signed envelope uses purpose `release-promotion`

### Requirement: Signed promotion receipt keyring verification
r[molten.evidence.release_promotion.signed_receipt_verify] Molten's dogfood release evidence MUST verify the signed release promotion receipt through the signed receipt keyring and fail the check when verification does not pass.

#### Scenario: Keyring verifies signed promotion receipt
- GIVEN a signed promotion receipt envelope and the dogfood signed receipt keyring
- WHEN signed receipt verification runs with the selected key id, signer, trust root, and purpose
- THEN verification passes
- AND the verification log is preserved with the dogfood release output

### Requirement: Release promotion summary record
r[molten.evidence.release_promotion.summary_record] Molten MUST emit a canonical `release-promotion-summary-v1` artifact that summarizes promotion receipt, signed promotion envelope, selected keyring key, source evidence, Octet evidence, Cairn evidence, diagnostics, and evidence-only checks.

#### Scenario: Summary binds promotion readback refs
- GIVEN a realized dogfood release output with promotion evidence
- WHEN release promotion summary generation runs
- THEN the summary binds the promotion receipt ref, signed envelope ref, signed subject ref, signed key ref, source ref, Octet ref, and Cairn ref

### Requirement: Release promotion summary CLI
r[molten.evidence.release_promotion.summary_cli] Molten MUST expose `molten dogfood release-promotion-summary` to write promotion summaries from a realized dogfood output and signed receipt keyring.

#### Scenario: CLI writes summary artifact
- GIVEN a dogfood release output and signed receipt keyring
- WHEN the summary CLI runs
- THEN it writes `release-promotion-summary-v1` to the requested output path
- AND prints a status line containing the summary decision and refs

### Requirement: Signed promotion summary readback
r[molten.evidence.release_promotion.summary_signed_readback] Release promotion summaries MUST verify the signed promotion envelope through the selected keyring key and require the signed subject ref to match the promotion receipt ref.

#### Scenario: Signed promotion readback passes
- GIVEN a passing promotion receipt and signed promotion envelope over that receipt
- AND a current unrevoked signed receipt key
- WHEN summary generation runs
- THEN the summary decision is `pass`
- AND it binds the signed envelope and key refs

### Requirement: Promotion summary denies stale or missing readback
r[molten.evidence.release_promotion.summary_deny_readback] Release promotion summaries MUST emit deny evidence when the promotion receipt is missing, not passing, bound to another output path, unsigned, signed for another subject, or signed by a missing/stale/revoked key.

#### Scenario: Missing signed promotion denies summary
- GIVEN a passing promotion receipt
- AND no signed promotion envelope in the dogfood output
- WHEN summary generation runs
- THEN the summary decision is `deny`
- AND diagnostics identify signed promotion verification failure

### Requirement: Promotion summary remains evidence only
r[molten.evidence.release_promotion.summary_evidence_only] Release promotion summaries MUST NOT grant authority, policy, provenance, resource, transport, source-gate, retention, destructive-operation trust, release publication authority, or permission to bypass subsystem gates.

#### Scenario: Summary does not replace subsystem gates
- GIVEN a passing release promotion summary
- WHEN a subsystem or release publisher requires its own authority or gate evidence
- THEN the summary MUST NOT be sufficient authority
- AND the subsystem MUST still require its own gate evidence

### Requirement: Signed promotion subject binding
r[molten.evidence.release_promotion.signed_subject_binding] Molten's dogfood release evidence MUST verify the signed release promotion receipt against the exact subject ref emitted by `molten dogfood release-promote`.

#### Scenario: Signed promotion subject matches emitted promotion receipt
- GIVEN a release promotion gate receipt emitted by dogfood release promotion
- AND a signed receipt envelope for release promotion
- WHEN signed promotion verification runs
- THEN verification requires the signed envelope subject ref to equal the emitted promotion receipt ref
- AND verification fails when the signed envelope subject ref differs

### Requirement: Signed promotion subject binding remains evidence only
r[molten.evidence.release_promotion.signed_subject_evidence_only] Signed promotion subject-ref binding MUST NOT grant authority, policy, provenance, resource, transport, source-gate, retention, destructive-operation trust, release publication authority, or permission to bypass subsystem gates.

#### Scenario: Subject binding does not grant publication authority
- GIVEN a signed promotion receipt whose subject ref matches the emitted promotion receipt
- WHEN release publication or a subsystem operation requires authority
- THEN the matching subject binding MUST NOT be treated as sufficient authority
- AND the subsystem MUST still require its own gate evidence

### Requirement: Signed promotion receipt remains evidence only
r[molten.evidence.release_promotion.signed_receipt_evidence_only] Signed release promotion receipts MUST NOT grant authority, policy, provenance, resource, transport, source-gate, retention, destructive-operation trust, release publication authority, or permission to bypass subsystem gates.

#### Scenario: Signed promotion is not release authority
- GIVEN a verified signed release promotion receipt
- WHEN a subsystem or release publisher requires its own authority or gate evidence
- THEN it MUST NOT treat the signed promotion receipt as sufficient authority
- AND it MUST still require the subsystem or publication gate evidence

### Requirement: Release promotion remains evidence only
r[molten.evidence.release_promotion.evidence_only] Release promotion receipts MUST NOT grant authority, policy, provenance, resource, transport, source-gate, retention, destructive-operation trust, or permission to bypass subsystem gates.

#### Scenario: Promotion pass does not replace subsystem gates
- GIVEN a passing release promotion receipt
- WHEN a subsystem performs privileged, destructive, transport, provenance-sensitive, source-gated, or retention-sensitive work
- THEN it still requires its own matching gate receipts and MUST NOT treat the promotion receipt as subsystem authority

### Requirement: Signed evidence remains evidence only
r[molten.evidence.signed_receipts.evidence_only] Signed receipt envelopes and signed release bundle member checks MUST NOT grant authority, policy, provenance, resource, transport, source-gate, retention, destructive-operation trust, or permission to bypass subsystem gates.

#### Scenario: Signature does not replace subsystem gates
- GIVEN a signed release evidence receipt passes verification
- WHEN a subsystem performs privileged, destructive, transport, provenance-sensitive, source-gated, or retention-sensitive work
- THEN it still requires its own matching gate receipts and MUST NOT treat the signed envelope as subsystem authority

### Requirement: Release export manifest
r[molten.evidence.release_export.manifest] Release evidence export manifests MUST bind a realized dogfood output path, the release promotion summary ref, deterministic member path/content refs, and evidence-only/no-authority checks in canonical Preserves.

#### Scenario: Manifest binds portable release evidence members
- GIVEN a realized dogfood output with pass promotion summary evidence
- WHEN an operator creates a release export manifest
- THEN the manifest records the output path ref, promotion summary ref, member refs, deterministic layout check, evidence-only check, and no-release-authority check

### Requirement: Release export archive
r[molten.evidence.release_export.archive] Release evidence archives MUST use deterministic member ordering and file metadata so the same manifest and member bytes produce a stable portable review artifact.

#### Scenario: Deterministic archive export
- GIVEN a release export manifest and its listed members
- WHEN an operator writes the release evidence archive
- THEN the archive contains the manifest and listed payload members with deterministic tar metadata and without using logs as primary evidence

### Requirement: Release export verification
r[molten.evidence.release_export.verify] Release export verification MUST recompute member refs from the archive and emit a pass/deny receipt instead of relying on archive command logs.

#### Scenario: Tampered export denies
- GIVEN a release evidence archive with a missing, extra, stale, or tampered payload member
- WHEN an operator verifies the archive
- THEN verification emits `release-export-verify-receipt-v1` with decision `deny` and diagnostics identifying the member binding failure

### Requirement: Release export dogfood
r[molten.evidence.release_export.dogfood] The Nix dogfood release check MUST emit a portable release evidence archive, manifest, and verification receipt while preserving the evidence-only boundary.

#### Scenario: Dogfood emits portable export evidence
- GIVEN the dogfood release flow has produced signed promotion and promotion summary evidence
- WHEN the Nix dogfood check completes
- THEN it emits `release-evidence.tar.zst`, `release-export-manifest.preserves`, and `release-export-verify.preserves` with pass verification and no release authority granted
