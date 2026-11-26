# Persistent node identity deltas

## ADDED Requirements

### Requirement: Rotation overlap is verification-only
r[molten.crypto_identity.overlap_verification_only] A key in rotation overlap MUST be usable only for verification. The core rotation model MUST NOT offer a sign-and-verify overlap mode, and overlap currentness MUST NOT satisfy the signing predicate used by sign planning.

#### Scenario: Signing with an overlap key is denied
- GIVEN a key whose currentness is overlap during rotation
- WHEN sign planning evaluates a signing request for that key
- THEN planning MUST deny with a typed diagnostic and produce no signature.

#### Scenario: Verification with an overlap key still passes
- GIVEN a signature made before rotation by a key now in overlap
- WHEN verification evaluates that signature within the overlap policy bound
- THEN verification MUST accept it.

### Requirement: Public key status carries the opaque handle reference
r[molten.crypto_identity.status_handle_reference] The canonical public key status record MUST include the opaque key handle reference under a versioned status label. The reference MUST NOT expose private key material.

#### Scenario: Status record binds the handle reference
- GIVEN a resolved key with an opaque handle reference
- WHEN the canonical public status record is produced
- THEN the record MUST contain that handle reference and no private key material.

#### Scenario: Rotated generations have distinct status records
- GIVEN two key generations with different handle references
- WHEN their public status records are produced
- THEN the records MUST differ in their handle reference.
