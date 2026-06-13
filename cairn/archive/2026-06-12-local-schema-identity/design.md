## Context

Molten stores and exchanges canonical Preserves values at trust boundaries. The project now has:

- `typed_storage` records tagged with schema refs and explicit migration recipes,
- `artifacts` as a local content-addressed registry with schema/dependency indexes,
- `upgrades` that can plan migrations and name moves with impact analysis,
- effect handles and executor preflights that increasingly need explicit input/output schema evidence.

The next slice is local schema identity: a shared way to decide whether an actual schema ref can satisfy an expected schema ref.

The broader `unison-schema-identity` Cairn frames the goal. This focused change is the implementation-oriented local slice that binds schema identity to the artifact registry and typed storage. Unison remains non-normative prior art; Molten does not adopt Unison's typechecker, syntax, hash format, UCM, or unique-type implementation.

## Goals

- Represent schema identity mode explicitly in canonical schema artifacts.
- Compute deterministic structural fingerprints over normalized Preserves schema/value-shape metadata.
- Keep unique schema identity tied to artifact refs and admitted alias evidence, not mutable names.
- Return structured compatibility decisions with receipt refs.
- Integrate exact/structural/alias/migration decisions into typed-storage writes, loads, and migrations.
- Use the artifact registry to search by structural fingerprint and nominal dependents.
- Provide CLI tools for local inspection and tests.

## Non-Goals

- Do not implement a full Preserves schema typechecker in this slice.
- Do not infer semantic equivalence from equal shape for `unique` schemas.
- Do not use mutable names, aliases, docs, or filesystem paths as schema identity.
- Do not bypass storage policy, capability, migration, or handler admission.
- Do not replace typed-storage migration recipes with automatic transformations.
- Do not implement choreography, effect-schema, or policy-contract integration beyond shared DTOs/receipts in the first slice.

## Schema identity artifacts

Introduce canonical schema identity records:

```preserves
<schema-identity-v1 "molten.schema.identity.v1"
  <mode "structural" | "unique" | "branded-structural">
  <schema <schema-ref>>
  <shape <normalized-shape-ref> <structural-fingerprint>>
  <brand <none> | <some <brand-ref>>>
  <metadata [<metadata-ref> ...]>
  <policy [<policy-ref> ...]>
  <evidence [<receipt-ref> ...]>
  <checks [<check "names-not-identity" "pass"> ...]>>
```

`schema-ref` is the artifact ref for the schema artifact or schema metadata record. The identity artifact ref is the canonical hash of the whole record. The structural fingerprint is a separate domain-separated hash over the normalized shape only.

## Normalized shape model

The first implementation can normalize a bounded Preserves shape representation rather than full schema language features. Examples:

```preserves
<shape "string">
<shape "u64">
<shape "record" "profile" [<field "name" <shape "string">> <field "age" <shape "u64">>]>
<shape "sequence" <shape "string">>
<shape "map" <shape "string"> <shape "bytes">>
<shape "any-preserves">
```

Normalization rules:

- canonical Preserves encoding only,
- deterministic field/variant ordering where order is not semantically meaningful,
- no Rust debug formatting,
- no filesystem paths or local registry names,
- no docs/display labels unless labels are schema structure,
- explicit domain separator `molten.schema.structural-fingerprint.v1`.

Full recursive/cyclic schema normalization can be added later with explicit cycle indices or artifact refs.

## Compatibility decisions

Compatibility is a canonical decision record:

```preserves
<schema-compatibility-v1 "molten.schema.compatibility.v1"
  <decision "exact-artifact-match" | "structural-match" | "brand-match" | "admitted-alias" | "migration-available" | "mismatch-requires-migration" | "denied-by-policy">
  <expected <schema-identity-ref> <schema-ref> <mode> <fingerprint>>
  <actual <schema-identity-ref> <schema-ref> <mode> <fingerprint>>
  <alias <none> | <some <schema-alias-ref>>>
  <migration <none> | <some <migration-recipe-ref>>>
  <policy [<policy-ref> ...]>
  <evidence [<receipt-ref> ...]>
  <checks [<check "unique-not-structural-by-default" "pass"> ...]>>
```

Compatibility rules:

1. Equal schema refs pass as `exact-artifact-match`.
2. Two `structural` schemas pass as `structural-match` when fingerprints match.
3. Two `branded-structural` schemas pass as `brand-match` only when brand refs and fingerprints match.
4. `unique` schemas pass only on exact artifact match or admitted alias.
5. A migration recipe can change a mismatch into `migration-available`, but execution still requires typed-storage migration admission.
6. Policy denial wins over all compatibility forms.
7. Missing or malformed identity evidence fails closed.

## Alias artifacts

Unique schema aliases are explicit evidence:

```preserves
<schema-alias-v1 "molten.schema.alias.v1"
  <from <schema-ref>>
  <to <schema-ref>>
  <scope "storage" | "effect" | "protocol" | "policy" | "global-local-fixture">
  <policy [<policy-ref> ...]>
  <evidence [<receipt-ref> ...]>
  <checks [<check "alias-is-not-name" "pass"> ...]>>
```

Alias records are directional unless policy explicitly admits symmetric use. They are not mutable names and do not rewrite either schema artifact.

## Typed-storage integration

Typed storage should call the schema identity layer before denying a schema mismatch:

- exact stored/expected schema refs remain the fast path,
- structural match may admit load/write only when both schemas declare `structural`,
- alias match may admit unique schema compatibility only with an alias artifact and policy/evidence refs,
- migration recipe remains required for incompatible changes,
- every non-exact decision is included in get/put/migrate receipt details,
- if identity artifacts are absent, current exact-ref fail-closed behavior remains.

This preserves existing safety while enabling explicit compatibility.

## Registry integration

The local artifact registry should index schema identity artifacts by:

- schema ref,
- identity mode,
- structural fingerprint,
- brand ref,
- alias from/to refs,
- policy/evidence refs.

Queries needed for this slice:

- find structurally equivalent schema identities by fingerprint,
- find unique aliases for expected/actual refs,
- list nominal dependents by schema ref through artifact dependency/schema indexes.

## Receipts

Schema identity receipts should bind:

- operation: `fingerprint`, `alias-admit`, `compatibility`, `storage-put`, `storage-get`, `migration-check`,
- decision and reason,
- expected/actual schema refs and identity refs,
- fingerprints and brand refs,
- alias/migration refs when used,
- policy/evidence refs,
- checks for unique/structural safety.

Receipts must be canonical Preserves values and ledger/artifact-registry classifiable.

## CLI

Add `molten test schema` commands:

- `identity` or `fingerprint` to create a schema identity artifact from a shape file/ref,
- `alias` to create an admitted alias artifact,
- `compat` to compare expected vs actual identity artifacts with optional alias/migration refs,
- `search-fingerprint` to query the local registry for matching structural fingerprints.

All commands should print full refs, not short ids.

## Tests and properties

Required tests:

- equal structural shapes have equal fingerprints despite metadata/name differences,
- unique schemas with equal shapes are incompatible without exact ref or alias,
- branded structural schemas require both same brand and same fingerprint,
- alias decisions are scoped and directional,
- typed-storage load admits structural compatibility and records receipt evidence,
- typed-storage load rejects unique mismatch unless alias or migration is present,
- migration recipes still bind source/target refs and do not become automatic transforms,
- Hegel properties for fingerprint determinism, alias directionality, and compatibility-result invariants.

## Open Questions

- How much of Preserves schema normalization should be delegated to `preserves-schema` versus Molten shape artifacts?
- Should aliases eventually live in a Raft-backed control-plane registry for multi-node cutover?
- Should schema identity artifacts be required for every typed-storage schema ref, or remain optional with exact-match fallback?
- What is the smallest safe recursive schema normalization model for protocol payloads?
