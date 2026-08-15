## ADDED Requirements

### Requirement: Core evidence domains use typed representations
r[molten.typed_domains.content_refs] Molten SHOULD represent canonical content refs, schema refs, receipt refs, artifact refs, and evidence refs with typed content-ref domains in pure Rust cores instead of raw strings when the value must be a BLAKE3 Preserves content ref.

#### Scenario: Malformed ref fails at parse boundary
- GIVEN an external boundary value containing `sha256:not-a-blake3-ref` where a content ref is required
- WHEN Molten parses the value into a typed DTO
- THEN parsing fails before the DTO reaches the pure core
- AND no canonical pass evidence is built from the malformed ref.

### Requirement: Decisions and check statuses are typed domains
r[molten.typed_domains.decisions] Molten SHOULD represent common decision and check-status vocabularies as typed domains or enums in pure cores rather than open strings.

#### Scenario: Unsupported decision is rejected
- GIVEN a receipt parser reads decision `maybe`
- WHEN the parser constructs the typed decision domain
- THEN construction fails
- AND the receipt cannot satisfy later evidence admission.

### Requirement: Stable identifiers use reviewed constructors
r[molten.typed_domains.identifiers] Molten SHOULD represent stable ids, schema ids, operation ids, replay classes, profile ids, and plugin ids with reviewed constructors that enforce their domain predicates before pure core evaluation.

#### Scenario: Empty operation id cannot enter core
- GIVEN a hostcall input with an empty operation id
- WHEN the input is parsed into the typed operation domain
- THEN parsing fails before hostcall admission logic runs.

### Requirement: Representative DTO migrations use typed domains
r[molten.typed_domains.migrated_dtos] Representative plugin, capability, chunk, evidence, and consensus DTO migrations SHOULD parse high-risk raw strings into typed domains before pure core evaluation when the domain predicate is known.

#### Scenario: Boundary parser converts before core admission
- GIVEN a migrated boundary parser receives a valid operation id, schema id, decision, or replay class
- WHEN it constructs the DTO for the pure core
- THEN the DTO stores the reviewed typed domain or validates with the same typed constructor before core admission.

### Requirement: Typed migrations preserve canonical external forms
r[molten.typed_domains.hash_stability] Migrating raw-string DTO fields to typed domains MUST preserve canonical Preserves and JSON external forms unless a change explicitly records a schema migration.

#### Scenario: Typed DTO emits same canonical bytes
- GIVEN a migrated record built from valid typed domains
- WHEN Molten renders it to canonical Preserves bytes
- THEN its canonical ref matches the pre-migration fixture or the migration note explains the intentional ref change.

### Requirement: Invalid domain values are negatively covered
r[molten.typed_domains.negative_domains] Every newly introduced typed domain SHOULD include positive parse/format tests and negative malformed-value tests.

#### Scenario: Unsupported replay class test fails closed
- GIVEN a replay class outside the admitted vocabulary
- WHEN the typed replay-class parser runs
- THEN it returns an error with a domain-specific diagnostic.
