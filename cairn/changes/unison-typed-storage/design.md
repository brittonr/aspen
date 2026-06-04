## Context

Molten's architecture already requires canonical Preserves boundaries and Redb-backed durable metadata. The storage layer now needs a typed contract: what does a stored value mean months later, after schemas, policies, protocols, and executable artifacts have changed?

Unison avoids many durable-storage problems by making code and definitions content-addressed. Molten can borrow the principle without serializing arbitrary functions or adopting Unison values. Persisted Molten values should carry references to schema/type artifacts and producing/consuming artifact identities, with explicit migration paths when the desired type changes.

## Goals

- Persist values in canonical, hashable, schema-tagged form.
- Return typed durable references that include schema/type identity and storage authority.
- Validate loads against expected schema/type refs and policy before exposing values to actors.
- Track producing artifact, migration history, and evidence refs for audit.
- Support large payloads with content refs rather than inline store records.
- Keep storage adapter effects behind effect manifests and handler admission.

## Non-Goals

- Do not persist raw Rust memory, vtables, pointers, closures, or nondeterministic debug strings.
- Do not promise automatic semantic migration between incompatible schemas.
- Do not expose database internals or SQL-like ambient access to actors.
- Do not make every dataspace assertion durable by default.
- Do not let storage load forge capabilities or authority not present in the durable record and admission context.

## Durable reference model

A typed durable reference should include:

- `storage_ref_id`: content hash or store-derived id over namespace, key, schema ref, and value hash.
- `namespace`: policy-scoped storage namespace.
- `key`: canonical key value or content ref.
- `schema_ref`: artifact id for the Preserves schema/type expected at write time.
- `value_ref`: inline canonical Preserves bytes for small values or content ref for large values.
- `producer_artifact_ref`: artifact that created or last migrated the value.
- `policy_refs`: storage policy, retention, encryption, and access rules.
- `evidence_refs`: write receipt, migration receipts, provenance, and review evidence.
- `version_vector` or monotonic revision where the storage adapter supports updates.

The reference is not authority by itself. A caller still needs an admitted storage capability to load or mutate it.

## Write path

A write request should:

1. Validate that the caller's artifact declares the storage write effect.
2. Validate capabilities and namespace policy through Basalt/Nickel/Trellis gates.
3. Check that the value conforms to the declared schema/type artifact.
4. Encode the value as canonical Preserves or store large bytes by content ref.
5. Write through the storage adapter.
6. Emit a Cairn receipt with value hash, schema ref, namespace, key, actor/execution id, and policy decision refs.

## Load path

A load request should:

1. Validate declared storage read effect and capabilities.
2. Fetch the durable record by namespace/key/ref.
3. Verify the value hash/content ref and receipt evidence.
4. Check that the caller's expected schema ref matches the stored schema ref or that an admitted migration path exists.
5. Decode into the caller's expected typed representation only after validation.
6. Emit a load receipt or denial receipt.

## Migrations

A migration recipe is an artifact with:

- source schema ref,
- target schema ref,
- transformer artifact id,
- declared effects and handler profile,
- preconditions and postconditions,
- policy refs,
- tests/transcripts,
- review and execution receipts.

Migrations may be eager, lazy-on-read, or explicit batch jobs. All modes must preserve the original value hash, migration artifact id, result value hash, and receipts.

## Snapshots and authority graphs

Actor/vat snapshots should persist object state plus references to authorities the object already held. Loading a snapshot must not mint new capabilities. Capability refs, revocation state, and attenuation metadata must be part of the durable record or re-resolved through a gatekeeper with receipts.

## Open Questions

- Should storage refs be content-addressed immutable records first, with mutable keys modeled as metadata pointers?
- Which Preserves schema subset is sufficient for first typed-load validation?
- Should migrations run inside Wasmtime, Steel, or native Rust first?
- How should encrypted-at-rest payloads interact with content addressing and deduplication?
