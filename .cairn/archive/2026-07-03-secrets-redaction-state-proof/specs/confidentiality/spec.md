## ADDED Requirements

### Requirement: Reveal and decrypt proof binds exact secret identity
r[molten.secrets_state_proof.exact_reveal_binding] Molten MUST prove that reveal and decrypt receipts expose plaintext refs only when the receipt binds the exact secret ref or encrypted-ref id, commitment, authority, policy, resource, effect-handle, and output ref for the requested access.

#### Scenario: Mismatched encrypted-ref id denies
- GIVEN a passing reveal receipt for encrypted-ref id `A`
- WHEN decrypt is requested for encrypted-ref id `B`
- THEN decrypt decision is `deny`
- AND no plaintext ref for `B` is exposed.

### Requirement: Redaction profile proof preserves gate semantics explicitly
r[molten.secrets_state_proof.redaction_profile_gate] Molten MUST prove that redacted outputs are gate-preserving only when a redaction transform receipt explicitly proves required gate evidence was preserved; diagnostic redaction profiles MUST remain diagnostic-only.

#### Scenario: Diagnostic redaction cannot satisfy gate
- GIVEN a diagnostic redacted repro bundle
- WHEN a gate check attempts to use it as pass evidence
- THEN the gate decision is `deny`
- AND diagnostics identify the non-gate-preserving profile.

### Requirement: Secret cleanup is retention gated
r[molten.secrets_state_proof.cleanup_retention_gate] Molten MUST prove that secret cleanup receipts require matching retention admission, apply, and execution evidence before removing encrypted refs, plaintext refs, or cleanup tombstones.

#### Scenario: Cleanup without retention denies
- GIVEN a secret cleanup request with no matching retention apply ref
- WHEN cleanup evaluates destructive mutation
- THEN cleanup decision is `deny`
- AND secret commitments and encrypted refs remain available for audit.
