# Schema Identity and Evolution

Schema identity binds an interpretation, but it does not authorize a migration. This article distinguishes Molten's canonical boundary-schema artifacts from its bounded `schema-identity-core` pilot. Familiarity with content references is assumed. The [pilot note](../../schema-identity-core-pilot.md) and [architecture](../../architecture.md) remain governing sources; this [Technical companion](../README.md) does not define a replacement identity scheme.

## Three identities with different jobs

A boundary value reference identifies the canonical Preserves value. A boundary schema reference identifies a canonical description of the expected representation. A typed-value reference binds a decoded value reference to a schema and family. The rail implements these as distinct computations rather than treating a schema's human-readable name as enough context.

`boundary_schema_artifact_value` constructs `preserves-boundary-schema-artifact-v1` with family, version, Preserves schema specification version, record label, schema identifier, arity, and field contracts. `boundary_schema_ref` canonically hashes that artifact. `validate_boundary_claimed_schema_ref` compares a supplied, syntactically checked reference with the computed one and rejects a stale or different reference. See the [schema artifact implementation](../../../src/preserves/parts/rail/p005/body.rs).

This mechanism makes the schema description itself identity-bearing. A change to a field contract is not hidden behind an unchanged display label: it changes the value that feeds schema-reference calculation. That statement concerns the explicit artifact projection, not arbitrary comments, Rust names, or unprojected implementation details.

The same source computes a typed-value reference from `preserves-boundary-typed-codec-v1`, carrying the family, schema reference, and decoded value reference. This is an interpretation binding. It is not the hash of the decoded value alone, and it is not evidence that application policy approved using that value.

## Validation is exact, not an evolution engine

`validate_boundary_schema` expects the selected record label with exactly the specified arity, then validates fields against their contracts. It computes a value reference and schema reference and reports `pass` only after those checks succeed. An unknown extra field is not silently tolerated by this record-shape check.

Consequently, “the producer added a field” does not automatically mean an older consumer can accept it. Compatibility must be established under the relevant selected schema and migration contract. The helper does not search a registry for a compatible interpretation, run a migration, or approve an upgrade. The existing [boundary tests](../../../src/preserves/parts/rail/tests/m000/p001/body.rs) include wrong labels, missing fields, extra fields, wrong schema identifiers, and stale schema references.

Canonical decoding alone remains insufficient. It establishes a byte representation, while a schema check establishes conformity to one selected representation contract. Neither decides whether changing an application's durable meaning is acceptable.

## What the nominal pilot actually admits

The separate [pilot adapter](../../../src/schema_identity_core_pilot.rs) supplies three explicit facts: `owner`, `local_key`, and `stable_member_id`. It derives a nominal lineage from the first two, then constructs a closed graph descriptor with a record root and one required text member. Its text bound is 64 bytes; the supplied admission limits permit 16 nodes and 32 members. These are the inspected pilot's bounds and shape, not general limits for every Molten schema.

The adapter's tests compare one profile with a published nominal fixture vector and establish that changing the owner changes the nominal identity. This is identity separation, not an access-control rejection: both owner variants in that test are admitted successfully and their schema IDs differ. The [pilot note](../../schema-identity-core-pilot.md) describes owner crossing as rejected “by identity separation”; that wording should be read in this narrow sense, not as evidence that the adapter denies constructing the second descriptor.

There is no basis here for equating the pilot's `schema_id().to_hex()` output with the existing Preserves boundary schema reference. They describe different identity constructions. The pilot explicitly retains product ownership of DTOs, registries, storage, policy, receipts, migration execution, and release authority, and disclaims full legacy parity or production cutover.

## Worked evolution scenario

Consider an illustrative profile with owner `owner:a`, lineage key `profile`, and stable member `field:value`. Moving the same member shape into owner `owner:b` changes nominal identity in the pilot. Structural similarity therefore does not establish nominal continuity.

Separately, suppose a boundary producer emits a new field while retaining an old record label. The rail's exact-arity validation rejects that value under the old schema. Renaming a file or changing a human description cannot repair the mismatch; the reviewer must identify the intended representation and the actual migration authority. These examples show two independent obstacles: nominal ownership continuity and representation compatibility. Neither is resolved by recomputing a hash and calling the result a migration.

## Verification and limits

Suggested review compares old and new schema artifact projections, the selected schema reference at each decoder, and any explicit migration evidence supplied by the owning subsystem. For the pilot, inspect owner and lineage facts rather than only comparing graph shape. Retain the distinction between a changed identity and an admission error.

The cited tests were inspected, not executed for this documentation task. Their finite cases do not prove general migration safety, semantic equivalence, rollback correctness, registry coverage, or authorization. No new migration algorithm or compatibility promise is introduced here.

## Sources

- [Schema identity core pilot and non-claims](../../schema-identity-core-pilot.md)
- [Canonical architecture](../../architecture.md#core-envelope-spine)
- [Nominal authority boundaries](../../nominal-authority-references.md)
- [Schema artifacts, validators, and typed-value references](../../../src/preserves/parts/rail/p005/body.rs)
- [Boundary schema negative cases](../../../src/preserves/parts/rail/tests/m000/p001/body.rs)
- [Nominal graph pilot and owner-separation test](../../../src/schema_identity_core_pilot.rs)
