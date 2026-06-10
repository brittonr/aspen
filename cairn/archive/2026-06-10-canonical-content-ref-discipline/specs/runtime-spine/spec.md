# Runtime Spine Delta: canonical content ref discipline

### Requirement: Shared canonical content-ref parsing
r[molten.runtime_spine.canonical_content_refs.shape] Molten MUST validate BLAKE3 content refs through a shared parser that accepts only `blake3:<64 lowercase hex chars>` for canonical content refs unless another algorithm is explicitly modeled.

#### Scenario: Malformed content ref is rejected
- GIVEN a ref that is empty, truncated, non-hex, uppercase, path-like, or uses an unsupported algorithm
- WHEN a runtime, node-control, evidence, storage, protocol, or catalog boundary parses the ref
- THEN Molten rejects the ref before using it as identity evidence

#### Scenario: Canonical value computes a content ref
- GIVEN a canonical Preserves value
- WHEN Molten computes the value identity
- THEN the resulting content ref is the BLAKE3 hash of the canonical Preserves bytes formatted by the shared ref helper

### Requirement: Content addressing is evidence, not trust
r[molten.runtime_spine.canonical_content_refs.not_trust] Molten MUST NOT treat a well-shaped content ref as authority, policy, provenance, source-gate, retention, resource, or transport trust.

#### Scenario: Plausible ref lacks authority
- GIVEN a request with a syntactically valid payload ref but no authority or policy evidence
- WHEN the request is admitted
- THEN Molten denies the request through the authority or policy gate despite the valid ref shape

#### Scenario: Transport ref remains evidence-only
- GIVEN a live transport receipt that binds a syntactically valid envelope ref
- WHEN downstream dispatch evaluates the request
- THEN dispatch still depends on node-control authority, resource, idempotency, provenance, and source-gate receipts

### Requirement: Materialized ref readback
r[molten.runtime_spine.canonical_content_refs.materialized_readback] Molten MUST distinguish ref shape validation from local materialization and MUST recompute refs from local bytes or canonical values when an operation claims the content is present locally.

#### Scenario: Missing materialized content denies
- GIVEN a well-shaped ref that is not present in the claimed local ledger, chunk store, ingress store, or runtime journal
- WHEN an operation requires local materialized content
- THEN Molten emits denial diagnostics or a denial receipt instead of accepting the ref string

#### Scenario: Tampered materialized content denies
- GIVEN local bytes stored under a claimed ref
- WHEN recomputing the canonical or domain-separated BLAKE3 ref yields a different ref
- THEN Molten rejects the content and records a stale or tampered-ref diagnostic before side effects

### Requirement: Node-control refs use canonical discipline
r[molten.runtime_spine.canonical_content_refs.node_control] Molten MUST parse node-control request refs, payload refs, ingress envelope refs, live transport receipt refs, and subreceipt refs with the shared content-ref discipline.

#### Scenario: Node-control rejects short fixture refs
- GIVEN a node-control request whose payload ref is `blake3:fixture`
- WHEN the request is parsed or admitted outside test-only fixture construction
- THEN Molten rejects the request as malformed before dispatch

#### Scenario: Live ingress binds materialized envelope identity
- GIVEN canonical live ingress bytes received from transport
- WHEN Molten stores the ingress envelope locally
- THEN the live receipt binds the envelope ref recomputed from canonical bytes and does not treat transport delivery as authority

### Requirement: Runtime values expose canonical refs
r[molten.runtime_spine.canonical_content_refs.runtime_values] Molten SHOULD expose canonical refs for runtime values, messages, assertions, observations, events, turn journals, and state snapshots wherever those records cross a runtime, replay, harness, evidence, or storage boundary.

#### Scenario: Turn journal refs are stable under replay
- GIVEN a deterministic runtime run and its replay under the same inputs
- WHEN turn journals and state snapshots are emitted
- THEN their canonical refs are identical between the original run and replay

#### Scenario: Runtime value ref avoids debug-format identity
- GIVEN two equal runtime values with the same canonical Preserves bytes
- WHEN their refs are computed
- THEN the refs match even if Rust debug formatting or allocation layout differs

### Requirement: Migration coverage for content-ref discipline
r[molten.runtime_spine.canonical_content_refs.negative_tests] Molten MUST test malformed refs, wrong-length refs, unsupported algorithms, valid-shaped missing content, and tampered local bytes for migrated boundaries.

#### Scenario: Negative ref matrix fails closed
- GIVEN a migrated boundary that accepts content refs
- WHEN tests supply malformed, missing, or tampered refs
- THEN the boundary fails closed and emits structured diagnostics without mutating protected state

r[molten.runtime_spine.canonical_content_refs.migration] Molten SHOULD migrate artifact registry, catalog, coordination, protocol session, service runtime, transcripts, provenance, redaction, secrets, and job DAG validators to the shared ref helper in bounded slices.

#### Scenario: Migrated module removes ad-hoc prefix checks
- GIVEN a module that previously accepted refs with ad-hoc `blake3:` prefix checks
- WHEN the module is migrated
- THEN parse failures and diagnostics come from the shared content-ref helper while the module preserves its separate policy and authority gates
