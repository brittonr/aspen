# Runtime Spine Delta: Domain Codec Façades

### Requirement: Domains own narrow canonical artifact façades
r[molten.modularity.codec_facades.domain_owned] Domains that emit or consume canonical Preserves artifacts SHOULD expose narrow domain-owned constructors and parsers instead of requiring callers to assemble raw record labels and field sequences through a broad codec helper.

#### Scenario: Caller uses domain façade
- GIVEN a domain receipt, manifest, admission, or envelope artifact is constructed by repository-owned code
- WHEN the construction site is reviewed after façade migration
- THEN the caller uses a domain-owned function or type that names the artifact kind and input contract
- AND low-level Preserves record assembly is hidden behind that façade unless the code is inside the codec layer itself

#### Scenario: Raw assembly remains contained
- GIVEN a call site still assembles a raw Preserves record directly
- WHEN reviewers inspect the call site
- THEN the code is either inside a codec/façade implementation or records an explicit migration exemption

### Requirement: Codec façade migration preserves canonical identity
r[molten.modularity.codec_facades.identity_preserving] Introducing a domain codec façade MUST preserve canonical Preserves bytes, BLAKE3 refs, schema labels, and parser decisions for existing artifact versions unless a separate versioned schema change owns the break.

#### Scenario: Existing valid artifact keeps ref
- GIVEN a representative valid artifact fixture for a migrated domain
- WHEN the artifact is reconstructed through the new façade
- THEN its canonical bytes and BLAKE3 ref match the pre-migration artifact

#### Scenario: Identity drift fails review
- GIVEN a façade migration changes canonical bytes for an existing artifact version
- WHEN focused identity tests run
- THEN the test fails unless the change is explicitly modeled as a new schema version with updated evidence

### Requirement: Constructors and parsers stay symmetric
r[molten.modularity.codec_facades.parser_symmetry] Domain codec façades SHOULD pair constructors with parsers or validators that reject malformed, stale, unsupported, or wrong-kind artifacts before downstream side effects.

#### Scenario: Valid constructed artifact parses
- GIVEN a valid artifact constructed by a domain façade
- WHEN the paired parser consumes it
- THEN parsing succeeds and returns typed domain data

#### Scenario: Malformed artifact is rejected
- GIVEN an artifact with a wrong label, missing required field, malformed content ref, unsupported version, or contradictory field
- WHEN the paired parser consumes it
- THEN parsing fails before authority, transport, storage, execution, or retention side effects occur

### Requirement: Codec dependency direction remains inward
r[molten.modularity.codec_facades.dependency_direction] Shared codec helpers MUST NOT depend on high-level runtime, node, retention, job, plugin, CLI, or adapter domains.

#### Scenario: High-level import is blocked
- GIVEN a shared codec helper imports a high-level domain module
- WHEN dependency-boundary validation runs
- THEN validation fails or records the violation before release evidence is promoted
