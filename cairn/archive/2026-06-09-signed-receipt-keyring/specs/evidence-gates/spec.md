## ADDED Requirements

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
