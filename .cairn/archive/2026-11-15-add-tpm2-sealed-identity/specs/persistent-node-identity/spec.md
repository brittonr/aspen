# TPM2 sealed transport identity

## ADDED Requirements

### Requirement: PCR policy sealed transport backend
r[molten.crypto_identity.tpm2_sealed_transport]

The system MUST identify Tpm2Sealed separately from ManagedSecret and CapabilityFile. The backend reference MUST bind the exact sealed public object, SHA256 PCR selection, provider and executable identities. Unseal MUST use retained process capabilities, bounded execution, no inherited environment, and memory-only secret delivery. Failure MUST return a typed fail-closed outcome without fallback or secret diagnostics. Only transport-purpose keys are eligible for this shell.

#### Scenario: Matching PCR state
- GIVEN an admitted policy-only sealed transport record in disposable swtpm
- WHEN the measured tool unseals under the matching PCR state
- THEN the real consumer loads the matching transport secret without writing plaintext to any consumer-owned file, and every regular file in its namespace is checked for the record, seed and plaintext schema.
- AND disposable software-provider device state is a separate authority outside that namespace, not a file excluded from its scan; this does not claim plaintext absence from emulator-owned state or qualify physical TPM storage.

#### Scenario: Altered PCR state
- GIVEN the same sealed object after an actual selected PCR extension
- WHEN the consumer attempts unseal
- THEN it returns typed UnsealDenied and does not read a capability-file key.

### Requirement: Exact backend rotation
r[molten.crypto_identity.tpm2_backend_rotation]

The existing rotation plan MUST admit CapabilityFile to Tpm2Sealed and the reverse only with exact source handle and destination backend identity. Completion MUST reject a next handle for another backend or generation. Identity and successful unseal MUST NOT grant membership or authority.

#### Scenario: Completion selects another backend or generation
- GIVEN an admitted rotation plan binds the exact source handle and destination backend identity
- WHEN completion supplies a next handle for another backend or generation
- THEN completion MUST reject it without granting membership or authority.
