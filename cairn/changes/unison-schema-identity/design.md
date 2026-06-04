## Context

Molten uses Preserves values at communication and storage boundaries. Preserves schemas and related type artifacts will become part of runtime evidence: payloads claim schemas, storage records are tagged with schema refs, effect handlers validate input/output schemas, and policy contracts declare accepted shapes.

A single compatibility rule is not enough. A `UserId` and `OrderId` may both be strings but should not be interchangeable. Conversely, generic result or option shapes may intentionally be shared by structure. Unison's unique and structural type modes provide the right conceptual split, adapted to Molten's artifact registry and Preserves boundary.

## Goals

- Let schema authors choose structural or unique identity per schema artifact.
- Compute deterministic structural fingerprints over normalized schema shape.
- Preserve nominal/unique identity for domain-specific schemas even when shapes match.
- Make compatibility decisions explicit in policy, receipts, and error messages.
- Support aliases and migrations without pretending incompatible schemas are equal.
- Allow semantic search to find structurally equivalent schemas and nominal dependents separately.

## Non-Goals

- Do not adopt Unison's typechecker or exact hash format.
- Do not infer semantic equivalence from structural equality for unique schemas.
- Do not make schema names authoritative identity.
- Do not allow schema compatibility decisions to bypass storage, protocol, or policy admission.
- Do not implement full dependent typing or arbitrary schema theorem proving in this change.

## Identity modes

Schema artifacts should declare an identity mode:

- `structural`: equivalent to any schema with the same normalized structural fingerprint.
- `unique`: equivalent only to the same schema artifact id or admitted explicit alias.
- `branded_structural` (optional later): structural shape plus a declared brand id for controlled newtype-like reuse.

Names, docs, display aliases, and project paths are metadata and do not affect structural fingerprints.

## Compatibility result

Compatibility checks should return a structured decision:

- `exact_artifact_match`
- `structural_match`
- `brand_match`
- `admitted_alias`
- `migration_available`
- `mismatch_requires_migration`
- `denied_by_policy`

The decision should include expected schema id, actual schema id, identity modes, structural fingerprints, policy refs, and receipt refs.

## Normalization

Structural fingerprints must be computed over canonical schema form:

- deterministic field/variant ordering where the schema language permits unordered maps,
- expanded references only where expansion is part of the declared equivalence rule,
- stable treatment of recursion/cycles,
- domain-separated hash prefix for schema fingerprints,
- no Rust debug formatting, allocation identity, filesystem paths, or names-as-identity unless explicitly in a unique brand.

## Integration points

Typed storage uses schema identity to decide whether a stored value can be loaded as the caller's expected type or requires an admitted migration.

Choreography payload registries use schema identity to validate payload tags and to detect when a protocol upgrade changes wire semantics.

Effect handlers use schema identity to validate request and response values.

Policy contracts use schema identity to ensure the decision input/output schemas match the reviewed contract artifact.

## Policy and evidence

Every trust-boundary schema compatibility decision should emit or reference evidence containing:

- expected and actual schema refs,
- identity mode of each schema,
- structural fingerprints,
- compatibility decision,
- migration or alias artifact if used,
- policy decision refs,
- Cairn receipt ref.

## Open Questions

- Should recursive structural schemas hash via explicit cycle indices, Merkle fixed points, or artifact references?
- Should `branded_structural` be implemented now or delayed until use cases demand it?
- How much of Preserves schema normalization should live in pure core vs registry tooling?
- Should schema aliases be Raft-backed control-plane state once consensus is available?
