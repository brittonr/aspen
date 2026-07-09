# Content-Addressed Chunk Store Delta: Semantic Boundary

### Requirement: Chunk store responsibilities are semantically separated
r[molten.chunk_store.modularity.boundaries] Chunk store implementation SHOULD separate model types, canonical manifest codec, pure verification, filesystem storage, index adapter, Iroh exchange, retention integration, lineage evidence, and shell orchestration.

#### Scenario: Chunk module ownership is clear
- GIVEN chunk store code is reorganized
- WHEN reviewers inspect the module layout
- THEN each module has an identifiable responsibility such as model, codec, verify, fs_store, index, exchange, retention, lineage, or shell

### Requirement: Chunk refactors preserve content identity
r[molten.chunk_store.modularity.identity_preserving] Chunk store modularity refactors MUST preserve canonical manifest bytes, chunk refs, lineage refs, and parser decisions for existing artifact versions unless a separate schema change owns the break.

#### Scenario: Manifest ref remains stable
- GIVEN a representative valid chunk manifest fixture
- WHEN the manifest is reconstructed through the extracted codec boundary
- THEN its canonical bytes and BLAKE3 ref match the pre-migration fixture

#### Scenario: Tampered chunk denies verification
- GIVEN a manifest whose referenced chunk bytes are missing or tampered
- WHEN the pure verifier evaluates the manifest and byte summaries
- THEN verification fails before publish, import, GC, or lineage evidence is promoted

### Requirement: Chunk destructive paths consume retention admission
r[molten.chunk_store.modularity.retention_boundary] Chunk deletion, GC, unpin, destructive index mutation, or tombstone emission MUST consume admitted retention evidence or an explicit non-destructive plan before mutating local chunk state.

#### Scenario: Missing retention admission blocks chunk deletion
- GIVEN a chunk or manifest is locally present but retention admission is missing or denied
- WHEN the chunk store plans deletion or GC
- THEN the plan denies or omits destructive effects

### Requirement: Chunk boundary changes include positive and negative tests
r[molten.chunk_store.modularity.tests] Chunk store boundary refactors SHOULD include positive identity and verification tests plus negative tests for tampered bytes, missing chunks, malformed manifests, stale lineage, or missing retention admission.

#### Scenario: Chunk tests cover identity and denial
- GIVEN a chunk boundary is extracted
- WHEN reviewers inspect focused tests
- THEN valid content identity and at least one denied malformed or destructive path are covered
