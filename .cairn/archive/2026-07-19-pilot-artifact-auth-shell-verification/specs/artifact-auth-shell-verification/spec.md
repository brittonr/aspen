## ADDED Requirements

### Requirement: Molten verifies exact standalone statement bytes

r[molten.artifact_auth_shell.exact_verification] Molten MUST map standalone statements in a pure core, sign only their canonical `artifact_auth.statement.v1` bytes through a purpose-bounded product key shell, and verify the independently reconstructed statement through the pinned `artifact-auth-ed25519` implementation.

#### Scenario: Legacy verification cannot substitute for standalone verification

- GIVEN a legacy Molten domain signature or `cryptographic_verification_passed` value without a valid signature over the exact standalone statement
- WHEN the shell evaluates standalone authentication
- THEN standalone cryptographic verification SHALL fail without weakening the legacy decision.

### Requirement: Shell evidence is public, bounded, and discriminating

r[molten.artifact_auth_shell.evidence] Molten MUST expose public statement, full-key, and signature identities, signature hex, stable cryptographic failure class, and dual-run compatibility while excluding private key bytes, backend locators, credentials, and ambient authority.

#### Scenario: Tamper and carrier substitution fail closed

- GIVEN statement, signature, key, preimage, or carrier identity drift
- WHEN the shell reconstructs and verifies evidence
- THEN it SHALL report the exact bounded failure or compatibility blocker and SHALL NOT report unexplained parity.

### Requirement: Product authority remains outside standalone verification

r[molten.artifact_auth_shell.authority] Molten MUST retain key generation/storage/signing permission, currentness, capability, membership, transport, runtime, deployment, evidence composition, lifecycle, and release authority while the pilot remains diagnostic.

#### Scenario: Real standalone verification passes without authority admission

- GIVEN the production shell verifies a valid exact standalone signature
- WHEN dual-run compatibility is evaluated
- THEN `legacy_authoritative` and `rollback_available` SHALL remain true, `standalone_authority_admitted` SHALL remain false, and all product-owned gates SHALL still apply.
