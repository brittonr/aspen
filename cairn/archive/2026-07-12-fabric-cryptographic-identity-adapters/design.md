## Context

Canonical node identities currently model source selection, permissions, drift, rotation, and redaction, but endpoint material and federation signatures are derived through deterministic local fixture algorithms. Production key operations must be effectful, while identity and admission semantics must remain replayable without private-key access.

## Decisions

### 1. Identity semantics remain pure; key operations use adapters

**Choice:** Pure code constructs signature domains, key-purpose requests, verification inputs, freshness facts, rotation plans, and receipt payloads. Adapter shells generate keys, resolve opaque handles, sign bytes, verify signatures, persist secrets, and query backend status.

**Rationale:** Deterministic policy and replay must not depend on secret bytes, CSPRNG state, filesystem access, or secret-manager APIs.

### 2. Secret bytes never cross the adapter boundary

**Choice:** Extensions and primitives receive opaque key handles plus public-key and backend metadata. Signing accepts a handle, purpose, domain, and canonical payload ref; it never returns or serializes private key material.

**Rationale:** A stringly key API would allow receipts, errors, or extension state to leak reusable secrets.

### 3. Key purposes and signature domains are distinct

**Choice:** Transport endpoint, federation origin, delegation, evidence signing, and authority keys have separate purpose classes. Canonical signed bytes bind schema, purpose, domain version, signer, trust-root or verifier context, and payload ref. Sharing a key across purposes requires explicit policy and remains visible in receipts.

**Rationale:** Cross-protocol signature reuse and confused-deputy verification must fail closed.

### 4. Ed25519/Iroh is the initial production profile, not a core type

**Choice:** The first live profile uses Ed25519-compatible Iroh key material, but canonical descriptors identify algorithm and profile version without exposing Iroh runtime objects. Additional algorithms require separately registered profiles and conformance evidence.

**Rationale:** The fabric needs production cryptography without making one library type canonical.

### 5. Deterministic fixture crypto is explicitly non-production

**Choice:** Existing BLAKE3 fixture signatures and deterministic endpoint derivation remain available only under test/simulation profile ids. Production admission rejects fixture algorithm ids, public-input-derived secrets, and synthetic conformance refs.

**Rationale:** Keeping fixtures supports replay while correcting the security and evidence claim boundary.

### 6. Rotation and revocation are currentness transitions

**Choice:** Rotation plans bind old/new public refs, purpose, generation, backend, policy, activation boundary, overlap policy, and revocation evidence. Adapters perform secret effects only after pure admission; stale handles and superseded generations deny signing and endpoint startup.

**Rationale:** Persisting a new key is not enough to establish which identity is current.

## Functional core / imperative shell split

- Pure core: purpose/domain validation, source selection, public identity construction, verification decisions over supplied cryptographic outcomes, freshness, drift, rotation, revocation, redaction, and receipt payloads.
- Shell: CSPRNG access, key generation, restricted persistence, managed-backend calls, signing, cryptographic verification, Iroh endpoint construction, and bounded adapter status collection.

## Dependencies

- Fabric durable-state, time/entropy, and transport port changes.
- Capability-rooted node state.
- Existing persistent identity, authority, peer-bootstrap, and evidence schemas.

## Risks / Trade-offs

- Managed secret backends may be unavailable during startup. Fail closed or use an explicitly admitted fallback; never generate silent replacement identity.
- Key-handle APIs can still leak metadata. Redact backend names, paths, tokens, and raw errors according to policy.
- Algorithm agility can create downgrade paths. Profiles are explicit and no fallback occurs without a reviewed migration.
