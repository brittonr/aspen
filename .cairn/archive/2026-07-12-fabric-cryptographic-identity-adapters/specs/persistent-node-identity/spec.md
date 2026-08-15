## ADDED Requirements

### Requirement: Cryptographic identity uses adapter-neutral ports
r[molten.crypto_identity.adapter_contract] Molten MUST expose versioned key-generation, key-resolution, signing, verification, public-key, and adapter-status operations through canonical adapter contracts using opaque key handles. Pure identity, admission, and extension code MUST NOT receive backend objects or private key bytes.

#### Scenario: Opaque signer handle is admitted
- GIVEN a production key-store profile resolves a current signing key for an admitted purpose
- WHEN a caller requests a signature over canonical payload identity
- THEN the adapter receives an opaque handle and returns public signature metadata
- AND no private key material crosses the adapter boundary.

#### Scenario: Raw private key request is denied
- GIVEN an extension asks a crypto adapter to return or serialize private key bytes
- WHEN adapter command validation runs
- THEN the request MUST deny before backend access
- AND diagnostics MUST identify the prohibited secret-material operation.

### Requirement: Production keys use admitted entropy and persistence
r[molten.crypto_identity.production_key_lifecycle] Molten MUST generate production key material only through an admitted cryptographic entropy profile and MUST persist or resolve it through a capability-rooted restricted store or reviewed managed secret backend. Missing, unsafe, malformed, or unavailable required key storage MUST deny startup rather than silently generate replacement identity.

#### Scenario: First boot creates a restricted key
- GIVEN policy permits first-boot generation and an admitted capability-rooted key store is available
- WHEN identity startup executes
- THEN the shell MUST generate cryptographic key material, persist it with the strongest admitted restrictions, and emit redacted public identity evidence.

#### Scenario: Required key backend is unavailable
- GIVEN policy requires a managed secret backend and no current key handle can be resolved
- WHEN identity startup executes
- THEN startup MUST deny before endpoint advertisement or replacement-key generation.

### Requirement: Key purposes and signature domains are separated
r[molten.crypto_identity.purpose_domain_separation] Molten MUST bind every signing and verification operation to an admitted key purpose, schema, domain version, signer public ref, verifier or trust-root context, and canonical payload ref. Transport, federation, delegation, evidence, and authority purposes MUST remain distinct unless reviewed policy explicitly admits key sharing.

#### Scenario: Federation signature verifies in its domain
- GIVEN a current federation-origin key signs a canonical inventory under the federation inventory domain
- WHEN verification uses the matching purpose, domain, signer, and payload ref
- THEN verification MAY pass and MUST bind those values in its receipt.

#### Scenario: Cross-purpose signature is rejected
- GIVEN a valid transport-purpose signature is presented as a federation delegation signature
- WHEN verification evaluates its purpose and domain
- THEN verification MUST deny even when the public key and payload bytes otherwise match.

### Requirement: Signature verification binds canonical Preserves identity
r[molten.crypto_identity.canonical_signature_binding] Molten MUST sign or verify a canonical domain value that commits to the canonical Preserves payload ref rather than Rust layout, debug text, rendered output, mutable path, raw ticket, or transport frame encoding.

#### Scenario: Equivalent canonical payload verifies
- GIVEN semantically equivalent payloads produce the same canonical Preserves ref
- WHEN the admitted domain value is signed and verified
- THEN both constructions MUST resolve to the same signed payload identity.

#### Scenario: Tampered payload fails verification
- GIVEN a signature whose declared payload ref differs from the received canonical payload ref
- WHEN verification runs
- THEN verification MUST deny before the payload can satisfy federation, evidence, or authority admission.

### Requirement: Fixture cryptography cannot satisfy production admission
r[molten.crypto_identity.fixture_profile_boundary] Molten MUST classify deterministic BLAKE3 fixture signatures, public-input-derived endpoint secrets, synthetic keys, and test-only verifier profiles as simulation or fixture evidence. Production startup, federation, evidence promotion, and remote authority gates MUST reject those profile ids.

#### Scenario: Fixture signature remains replayable
- GIVEN a deterministic test profile explicitly selects fixture cryptography
- WHEN a replay suite signs and verifies a fixture payload
- THEN the fixture MAY pass within that test profile
- AND its evidence MUST identify the non-production algorithm and caveats.

#### Scenario: Fixture key is rejected in production
- GIVEN production startup receives a deterministic endpoint key derived from public identity fields
- WHEN crypto-profile admission runs
- THEN admission MUST deny before opening the live endpoint.

### Requirement: Rotation and revocation fence stale handles
r[molten.crypto_identity.rotation_revocation] Molten MUST model key rotation and revocation as explicit currentness transitions binding purpose, old and new public refs, backend, generation, activation boundary, policy, overlap posture, and evidence. Stale, revoked, superseded, wrong-generation, or wrong-purpose handles MUST deny signing and endpoint startup.

#### Scenario: Admitted rotation activates a new key
- GIVEN a rotation plan has current policy, backend, old-key, new-key, and activation evidence
- WHEN the activation transition commits
- THEN subsequent signing MUST use the new generation
- AND old-generation use MUST follow the declared overlap or revocation posture.

#### Scenario: Stale key handle is fenced
- GIVEN a key generation has been superseded without an admitted overlap window
- WHEN delayed work submits the old handle for signing
- THEN the adapter MUST deny without producing a signature.

### Requirement: Identity receipts and readback redact secrets
r[molten.crypto_identity.redaction] Molten MUST restrict identity, signing, verification, rotation, denial, startup, and operator records to public identities, opaque handle refs, source classes, purpose, profile, generation, permission status, redacted backend metadata, decisions, and diagnostics. Receipts MUST NOT contain private keys, seeds, bearer tokens, raw tickets, secret backend credentials, or sensitive host paths.

#### Scenario: Production startup receipt is public-only
- GIVEN startup resolves a valid persisted endpoint key
- WHEN it emits identity and adapter receipts
- THEN the receipts MUST include public identity and redacted source metadata
- AND MUST exclude the persisted secret and raw backend locator.

### Requirement: Crypto adapters have positive and negative conformance
r[molten.crypto_identity.adapter_conformance] Molten MUST run shared conformance for each admitted production or simulation crypto profile, including generation, restart stability, signing, verification, purpose separation, rotation, revocation, redaction, malformed input, unavailable backend, unsafe permissions, stale handles, and wrong-key behavior.

#### Scenario: Production adapter passes conformance
- GIVEN an adapter declares a production Ed25519-compatible profile
- WHEN shared conformance runs
- THEN it MUST pass positive generation, persistence, signing, verification, restart, rotation, and redaction cases
- AND every required negative case MUST fail closed.

#### Scenario: Secret-leaking adapter is denied
- GIVEN an adapter returns private bytes or includes them in diagnostics
- WHEN conformance inspects adapter outputs
- THEN production admission MUST deny that profile.
