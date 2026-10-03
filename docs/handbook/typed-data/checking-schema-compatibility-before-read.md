# Checking schema compatibility before a read
Mode: How-to

## Goal and prerequisites

Determine whether a consumer can interpret an existing typed-storage value without confusing schema compatibility, migration execution, or load authority. This procedure is source-checked and was not executed for this handbook batch. It uses APIs and checked-in cases rather than inventing a CLI workflow.

Before making a decision, obtain the actual stored schema reference, the consumer's expected schema reference, both identity artifacts when compatibility is needed, the caller's independently obtained admission, and the relevant application contract. Work from already captured evidence or an approved isolated store. `get_value` and related helpers may create directories/tables and persist receipts, so their names do not promise read-only diagnostics.

See the [Handbook](../README.md) for related workflows and [Schema Identity and Evolution](../../technical/envelopes/schema-identity-and-evolution.md) for the theory. The [architecture](../../architecture.md) remains the authority for canonical identity and effect separation.

## 1. Decide which schema construction you have

Do not compare human labels or assume every schema reference uses the same projection. Typed storage's inferred schema hashes a `storage-schema-artifact-v1` containing a Preserves value class. A `record` classification is not a checked record label, field count, or field-type contract. Boundary-schema validation described in the technical companion is a different mechanism.

Record the construction alongside each reference. If all you possess is a Rust type name or rkyv layout, stop: neither supplies the canonical schema artifact needed for this comparison. If application-level field checks are required, identify their owning validator before proceeding.

## 2. Compare the ordered pair of schema references

Treat expected and actual as directional roles. For equal references, the ordinary get binding check accepts the schema directly. This does not evaluate a separate compatibility policy record. The caller still needs admission and a valid payload; equality is not authority.

For unequal references, do not remove `expected_schema_ref` merely to make a read succeed. The [storage binding implementation](../../../src/typed/parts/storage/p002/body.rs) accepts absence of an expectation, but doing so abandons this consumer check. Instead, decide whether a reviewed compatibility artifact or an actual migration is appropriate.

## 3. Inspect identity modes and decision precedence

The [identity implementation](../../../src/schema/parts/identity/p000/body.rs) supports `structural`, `unique`, and `branded-structural`. Identity parsing recomputes normalized shape references and structural fingerprints; mismatches are errors. Branded structural identities require a brand; other modes cannot carry one.

The [decision function](../../../src/schema/parts/identity/p001/body.rs) checks policy denial first, then exact schema equality, directional alias, structural match, branded structural match, migration availability, and finally mismatch. Structural matching requires both modes to be structural. Brand matching requires both branded modes, equal brands, and equal fingerprints. Equal shape does not make two unique schemas interchangeable.

Capture the decision and inputs, not just a passing receipt. Policy/evidence references in an artifact do not by their presence demonstrate that an external policy authority approved its use.

## 4. Review aliases and migration availability explicitly

An alias must point from actual schema to expected schema. The implementation recognizes several scope labels, but its decision function accepts any recognized alias scope; the storage admission helper then checks the schema pair and decision without independently enforcing a storage-specific alias scope. This is a source-review limitation, not a reproduced runtime incident. Require the caller's reviewed scope selection rather than assuming the helper provides that boundary.

Also, `compatibility_admits_storage` currently treats `migration-available` as admitting. `get_value_with_schema_compatibility` does not execute that migration: it returns the existing canonical value after compatibility and integrity checks. If the consumer requires transformed data, stop and use a separately reviewed execution plan. A migration reference is not a completed transformation.

## 5. Bind the artifact to this read

For the compatibility path, `SchemaCompatibilityGetInput` carries root, namespace, key, expected schema reference, compatibility value, and admission. The helper rejects an artifact whose expected/actual schema references do not match this request. A successful receipt includes the compatibility value and its generated compatibility receipt in details.

There is an important trust boundary: parsing a supplied compatibility record checks its format and selected fields; it does not rerun the original decision from independently retrieved identity, alias, and policy artifacts. Retain producer provenance and the reviewed construction path. Never treat an arbitrary well-formed passing record as self-authorizing evidence.

## Worked decision: the profile fixture

The [integration fixture](../../../src/typed/parts/storage/tests/m000/p000/body.rs) stores `<profile "alice" 7>` and constructs two structural identities with a `profile` shape containing `name` and `age`. Their schema references differ, but the matching structural shape yields a compatible read. The [alternate case](../../../src/typed/parts/storage/tests/m000/p001/body.rs) switches to unique identities: the mismatch is denied, then a directional storage alias permits the read.

Use this case to distinguish three results in a review: exact equality needs no compatibility path; structural agreement requires the proper modes; unique identity needs an admitted relationship rather than a matching shape. The fixture supplies local references and does not prove deployment policy, general migration safety, or successful execution in this batch.

## Sources

- [Handbook](../README.md)
- [Canonical architecture](../../architecture.md)
- [Schema Identity and Evolution](../../technical/envelopes/schema-identity-and-evolution.md)
- [Identity artifacts and fingerprints](../../../src/schema/parts/identity/p000/body.rs)
- [Compatibility parsing and precedence](../../../src/schema/parts/identity/p001/body.rs)
- [Storage compatibility consumer](../../../src/typed/parts/storage/p002/body.rs)
- [Structural fixture](../../../src/typed/parts/storage/tests/m000/p000/body.rs)
- [Unique and alias fixture](../../../src/typed/parts/storage/tests/m000/p001/body.rs)
