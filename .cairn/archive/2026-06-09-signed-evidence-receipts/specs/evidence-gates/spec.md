# Evidence Gates Delta: signed evidence receipts

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

### Requirement: Signed evidence remains evidence only
r[molten.evidence.signed_receipts.evidence_only] Signed receipt envelopes and signed release bundle member checks MUST NOT grant authority, policy, provenance, resource, transport, source-gate, retention, destructive-operation trust, or permission to bypass subsystem gates.

#### Scenario: Signature does not replace subsystem gates
- GIVEN a signed release evidence receipt passes verification
- WHEN a subsystem performs privileged, destructive, transport, provenance-sensitive, source-gated, or retention-sensitive work
- THEN it still requires its own matching gate receipts and MUST NOT treat the signed envelope as subsystem authority
