## ADDED Requirements

### Requirement: Preserves boundary codecs are typed and schema-backed
r[molten.preserves_boundary_codegen.typed_codecs] Molten SHOULD provide typed or schema-backed codec wrappers for adopted Preserves boundary records that cross runtime, storage, transport, plugin, policy, or evidence boundaries. The codecs MUST preserve the canonical public record label, field order, schema id, and value ref unless an explicit compatibility migration says otherwise.

#### Scenario: Typed codec preserves public value ref
- GIVEN a valid existing Preserves boundary fixture
- WHEN the adopted typed codec parses and unparses the fixture
- THEN the canonical value ref is unchanged
- AND semantic admission sees the same schema id and field values.

#### Scenario: Malformed typed input denies before semantic admission
- GIVEN a boundary value with the right label but a malformed typed field
- WHEN the typed codec evaluates the value
- THEN the codec returns a deny diagnostic
- AND semantic admission and side effects are not invoked.

### Requirement: External Preserves bytes decode strictly before admission
r[molten.preserves_boundary_codegen.strict_decode] Molten MUST decode external Preserves bytes by requiring the original bytes to equal the canonical packed re-encoding before schema validation, semantic admission, durable import, transport delivery, plugin hostcall evaluation, or receipt acceptance.

#### Scenario: Non-canonical external bytes deny
- GIVEN external bytes that parse as a Preserves value but re-encode to different canonical packed bytes
- WHEN the boundary receives the bytes
- THEN strict canonical decode is `deny`
- AND no schema, authority, provenance, policy, resource, transport, ledger, retention, plugin, or execution side effect is admitted.

#### Scenario: Canonical bytes bind the value ref
- GIVEN canonical packed bytes for an admitted boundary value
- WHEN strict canonical decode succeeds
- THEN Molten binds the decoded value ref to the BLAKE3 hash of those bytes
- AND downstream checks use that ref rather than rendered text or debug output.

### Requirement: Boundary receipts bind schema and value refs
r[molten.preserves_boundary_codegen.schema_ref_evidence] Molten MUST make adopted boundary validation evidence name the schema family, schema version or schema artifact ref, decoded value ref, codec decision, and diagnostics. Rendered logs MUST NOT replace this canonical evidence.

#### Scenario: Receipt identifies schema artifact
- GIVEN an adopted schema-backed boundary value passes validation
- WHEN Molten emits validation or admission evidence
- THEN the evidence names the schema family, schema artifact ref, decoded value ref, and pass decision.

#### Scenario: Missing schema evidence remains diagnostic
- GIVEN a value is well-formed but no required schema artifact ref is available for an adopted high-risk boundary
- WHEN validation evidence is required
- THEN validation denies or remains diagnostic-only according to the boundary policy
- AND the value is not treated as authority or provenance.

### Requirement: Preserves routing patterns have canonical ASTs
r[molten.preserves_boundary_codegen.pattern_ast] Molten MUST define a bounded canonical Preserves pattern AST for dataspace routing and policy-visible matching. The AST MUST have deterministic serialization, deterministic binding order, explicit unsupported-form denials, and resource bounds.

#### Scenario: Pattern AST ref is stable
- GIVEN the same supported routing pattern on two peers
- WHEN each peer parses it into the canonical pattern AST
- THEN both peers produce the same AST ref
- AND the same candidate value produces the same match decision and binding order.

#### Scenario: Unsupported pattern denies before routing
- GIVEN a routing pattern that uses an unsupported compound, unbounded wildcard, or ambiguous binding form
- WHEN the pattern is parsed
- THEN parsing denies with a deterministic diagnostic
- AND the pattern cannot route messages, assertions, retractions, policy decisions, or side effects.

### Requirement: Dataspace routing uses adopted Preserves patterns
r[molten.preserves_boundary_codegen.pattern_routing] Molten SHOULD route local dataspace `Observe` subscriptions through the adopted Preserves pattern AST rather than equality-only matching where the pattern feature is enabled. Initial assertion delivery, future assertion delivery, and retraction delivery MUST remain deterministic and owner-scoped.

#### Scenario: Observe pattern sees current and future matches
- GIVEN a local dataspace contains current assertions and a supported Observe pattern is registered
- WHEN the pattern matches current and later future assertions
- THEN the observer receives deterministic initial and future deliveries bound to assertion refs, owner refs, pattern ref, and route result ref.

#### Scenario: Retraction follows the same pattern route
- GIVEN an observer is subscribed through an adopted pattern
- WHEN a matching owner-scoped assertion is retracted
- THEN the observer receives a deterministic retraction delivery
- AND non-matching assertions are not delivered.

### Requirement: Preserves boundary fixture corpus covers pass and deny paths
r[molten.preserves_boundary_codegen.fixture_corpus] Molten MUST maintain canonical positive and negative fixtures for adopted Preserves boundary codecs, strict decode paths, schema ref binding, and routing patterns.

#### Scenario: Positive fixture roundtrips canonically
- GIVEN a positive boundary fixture in the corpus
- WHEN the fixture is parsed, typed, unparsed, and canonicalized
- THEN the expected value ref, schema ref, and pass decision are reproduced.

#### Scenario: Negative fixture denies before side effects
- GIVEN a negative fixture with non-canonical bytes, wrong label, missing field, malformed ref, unsupported version, missing schema ref, or unsupported pattern
- WHEN the adopted boundary evaluates it
- THEN the decision is deny or diagnostic-only as specified
- AND no side effect is admitted.

### Requirement: Schema validation does not grant authority
r[molten.preserves_boundary_codegen.no_schema_authority] Molten MUST treat schema validation, typed codec success, pattern parse success, and strict canonical decode success as evidence about value shape and identity only. They MUST NOT grant authority, policy, provenance, resource, transport, source-gate, retention, plugin, or execution trust.

#### Scenario: Shape-valid privileged request still needs authority
- GIVEN a privileged request whose Preserves bytes are canonical and whose schema validation passes
- WHEN the request lacks required authority, policy, resource, provenance, or source-gate evidence
- THEN semantic admission denies before the privileged side effect
- AND diagnostics state that schema or codec pass evidence does not grant authority.
