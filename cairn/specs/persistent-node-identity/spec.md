# Persistent Node Identity Specification

## Purpose

Defines the `persistent-node-identity` capability.

## Requirements

### Requirement: System MUST Define canonical node identity records with node id, endpoint public key, key source class, data-dir/secret-backend ref, policy refs, and receipt refs
r[molten.node_identity.record_model] The system MUST Define canonical node identity records with node id, endpoint public key, key source class, data-dir/secret-backend ref, policy refs, and receipt refs.

### Requirement: System MUST Implement explicit-key, persisted-file, generate-and-persist, and deny-if-unavailable resolution order
r[molten.node_identity.resolution_order] The system MUST Implement explicit-key, persisted-file, generate-and-persist, and deny-if-unavailable resolution order.

### Requirement: System MUST Document and enforce that stable node identity grants no capabilities by itself
r[molten.node_identity.no_identity_authority] The system MUST Document and enforce that stable node identity grants no capabilities by itself.

### Requirement: System MUST Ensure startup receipts never expose private key material
r[molten.node_identity.secret_redaction] The system MUST Ensure startup receipts never expose private key material.

### Requirement: System MUST Persist generated endpoint secret material to configured node data directory with restricted permissions where supported
r[molten.node_identity.file_backend] The system MUST Persist generated endpoint secret material to configured node data directory with restricted permissions where supported.

### Requirement: System MUST Detect endpoint id drift and deny or require admitted recovery/key-rotation policy
r[molten.node_identity.drift_detection] The system MUST Detect endpoint id drift and deny or require admitted recovery/key-rotation policy.

### Requirement: System MUST Emit receipts for identity resolution, first boot generation, load, drift, denial, and rotation
r[molten.node_identity.startup_receipts] The system MUST Emit receipts for identity resolution, first boot generation, load, drift, denial, and rotation.

### Requirement: System MUST Validate node identity config through Nickel/static policy before startup side effects
r[molten.node_identity.config_contract] The system MUST Validate node identity config through Nickel/static policy before startup side effects.

### Requirement: System MUST Include node identity refs in peer bootstrap handshakes and join admission
r[molten.node_identity.peer_bootstrap] The system MUST Include node identity refs in peer bootstrap handshakes and join admission.

### Requirement: System MUST Include node identity refs in replay/startup evidence without requiring private key access
r[molten.node_identity.replay_refs] The system MUST Include node identity refs in replay/startup evidence without requiring private key access.

### Requirement: System MUST Add tests proving restart with the same data dir preserves endpoint id
r[molten.node_identity.restart_tests] The system MUST Add tests proving restart with the same data dir preserves endpoint id.

### Requirement: System MUST Add Hegel property tests for resolution determinism and no-secret-in-receipt invariants
r[molten.node_identity.property_tests] The system MUST Add Hegel property tests for resolution determinism and no-secret-in-receipt invariants.

### Requirement: Iroh endpoint secret backends are explicit
r[molten.node_identity.iroh_secret_backend_model] Molten MUST model Iroh endpoint identity sources as explicit key metadata, managed secret backend metadata, persisted node-state file metadata, generate-and-persist policy, or fail-closed denial policy, and MUST bind the selected source class into startup identity receipts without exposing private key material.

#### Scenario: Startup selects persisted endpoint key
- GIVEN a node state root contains an admitted persisted Iroh endpoint secret and matching public identity metadata
- WHEN startup identity resolution runs
- THEN the decision selects the persisted-file source class and emits a receipt that binds only redacted source metadata and the endpoint public identity ref.

#### Scenario: Missing required backend denies
- GIVEN profile policy requires a managed secret backend and no usable backend metadata is available
- WHEN identity resolution runs
- THEN startup denies before generating a replacement key or advertising an endpoint.

### Requirement: Iroh identity resolution is pure before secret IO effects
r[molten.node_identity.iroh_identity_resolution_core] Molten MUST decide Iroh identity source selection with a pure core over explicit metadata facts, expected node identity refs, policy refs, and platform capability facts before shell code generates keys, persists files, rotates identity, or accepts drift.

#### Scenario: First boot generation is policy admitted
- GIVEN no explicit key, backend key, or persisted file exists and profile policy permits first-boot generation
- WHEN the pure resolution core evaluates the metadata facts
- THEN it returns a generate-and-persist decision with receipt inputs and required permission expectations.

#### Scenario: Malformed key metadata denies
- GIVEN explicit or persisted key metadata is malformed or inconsistent with the expected endpoint public identity
- WHEN the pure resolution core evaluates it
- THEN the decision denies with diagnostics before the live endpoint is started.

### Requirement: Persisted Iroh secrets use restricted permissions
r[molten.node_identity.iroh_identity_permissions] Shell-owned Iroh endpoint secret persistence MUST use owner-only permissions or the strongest supported platform restriction, and MUST emit deterministic diagnostics when requested restrictions cannot be verified or applied.

#### Scenario: Owner-only file is accepted
- GIVEN a generated Iroh endpoint secret is persisted in the node state root with owner-only access on a platform that supports it
- WHEN startup verifies the persisted source metadata
- THEN the identity receipt records a restricted-permission pass without revealing the secret bytes.

#### Scenario: Unsafe permissions deny or require policy
- GIVEN a persisted Iroh endpoint secret is readable outside the owner boundary
- WHEN startup identity resolution evaluates it
- THEN startup denies or requires explicit recovery policy before using the secret.

### Requirement: Endpoint identity drift requires admitted recovery
r[molten.node_identity.iroh_identity_drift_recovery] Molten MUST detect when an existing node identity scope observes a different Iroh endpoint public key than the prior admitted identity record, and MUST deny startup or require admitted rotation/recovery evidence before accepting the changed endpoint identity.

#### Scenario: Restart preserves endpoint identity
- GIVEN a node has an admitted identity record and persisted endpoint secret
- WHEN the node restarts with the same state root and no rotation policy
- THEN startup observes the same endpoint public identity and emits a load receipt bound to the prior identity ref.

#### Scenario: Unadmitted drift denies startup
- GIVEN a node identity record binds one endpoint public key and startup observes a different endpoint public key for the same node scope
- WHEN no admitted rotation or recovery receipt is supplied
- THEN startup denies before advertising the new endpoint.

### Requirement: Iroh identity receipts redact secrets
r[molten.node_identity.iroh_identity_receipt_redaction] Startup, replay, peer-bootstrap, drift, denial, and rotation receipts that mention Iroh identity MUST bind endpoint public identity refs, source class, policy refs, permission diagnostics, prior/new identity refs where applicable, and redacted source metadata, and MUST NOT include private key bytes, bearer tokens, raw ticket material, or sensitive local path contents.

#### Scenario: Receipt exposes public identity only
- GIVEN startup loads a valid endpoint secret
- WHEN the startup identity receipt is emitted
- THEN the receipt includes the endpoint public identity ref and source class
- AND the receipt excludes private key bytes and raw credential material.

#### Scenario: Replay does not require private key
- GIVEN replay validates startup identity evidence
- WHEN replay inspects the identity receipt
- THEN replay uses public identity refs and receipt bindings without requiring access to the private endpoint key.

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
