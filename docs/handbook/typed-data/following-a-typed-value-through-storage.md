# Following a typed value through storage
Mode: Walkthrough

This source-only walkthrough follows the checked-in `roundtrip_schema_tagged_preserves_value` fixture from its Preserves input to its stored binding, read result, and receipts. Use it to identify what evidence a storage experiment should preserve before attempting a migration. It is not a production deployment recipe. The fixture and implementation were source-checked, not executed for this handbook batch; no terminal output or generated digest is claimed.

The [Handbook](../README.md) supplies adjacent workflows. For identity theory, use [Schema Identity and Evolution](../../technical/envelopes/schema-identity-and-evolution.md); for the governing identity boundary, use the [architecture](../../architecture.md).

## 1. Identify the fixture inputs

Open the [roundtrip fixture](../../../src/typed/parts/storage/tests/m000/p000/body.rs). It parses the concrete value `<profile "alice" 7>`, selects namespace `profiles` and key `alice`, and supplies no explicit schema reference. Producer, policy, and evidence references come from test helpers. `Admission::local_fixture("roundtrip")` creates fixture references; it does not obtain deployment authority.

The observable input boundary is a `PutInput`, not a serialized Rust struct. Keep the original Preserves value, namespace/key, and provenance together when tracing an actual caller. Do not copy the fixture admission into an operational request and present it as authorization.

## 2. Follow schema selection before payload persistence

The [put implementation](../../../src/typed/parts/storage/p001/body.rs) creates directories, validates inputs and admission shape, rejects executable-authority markers, and computes an inferred schema. An explicit declared schema must equal that inferred reference or the put stores a denial receipt and returns an error.

The [inference helper](../../../src/typed/parts/storage/p009/body.rs) projects only the Preserves value class into `storage-schema-artifact-v1`. This example is a `record`. It does not infer a complete `profile` contract with a string name and integer age. An equal inferred schema therefore is not evidence that an application's field-level validation has happened.

The useful checkpoint is the selected `schema_ref` and its inference mode. The fixture later checks `inferred-preserves-value-class`. If a consumer requires a richer schema, record that separate validation requirement rather than quietly attributing it to this inference step.

## 3. Separate value identity from storage identity

The put path computes canonical bytes and a canonical value hash. Canonical Preserves plus BLAKE3 defines this identity; Rust layout and any future archive layout do not.

Payload selection compares canonical byte length with `INLINE_VALUE_LIMIT`, which is 4096 bytes. The short profile fixture follows the inline path. Larger values use a chunk-store object of kind `typed-storage-value`, represented by a manifest reference and length. The separate [large-value fixture](../../../src/typed/parts/storage/tests/m000/p001/body.rs) exercises that alternative and deliberately corrupts a chunk to require rejection; its corruption step is a test action, not an operator repair technique.

The typed storage entry binds schema, value, payload description, revision, producer, and other references. Hashing that entry produces `storage_ref`. The value reference identifies the canonical value; the storage reference identifies this binding. Preserve both, since a schema rename can retain value identity while changing storage identity.

## 4. Locate the commit boundary

`persist_entry` updates `typed_storage_records_v1`, keyed by the canonical namespace/key reference, and `typed_storage_refs_v1`, keyed by storage reference. For inline data it also updates `typed_storage_inline_values_v1`, keyed by value reference. The put receipt is inserted in the same Redb write transaction before commit.

The backing database is `typed-storage.redb`; the chunk subtree is `chunks`. These are local implementation locations, not content identities or permission tokens. Chunk persistence occurs before this index transaction, so the transaction alone is not proof of atomicity across every backend effect. The fixture makes no distributed consistency or crash-recovery claim.

## 5. Trace the read and its independent checks

The fixture calls `get_value` with the schema returned by put. The [get implementation](../../../src/typed/parts/storage/p002/body.rs) loads the indexed binding, parses it, checks the expected schema, constructs effect evidence, reads the payload, parses canonical bytes, and compares the resulting value hash with the stored value reference.

Success returns the value and a get receipt. The fixture asserts value equality, storage-reference equality, the inferred identity mode, and the Redb handler profile. It also invokes `verify_ref` and lists receipts. These assertions describe intended observable boundaries, not measurements made during this documentation task.

A get is not a read-only forensic probe: it can initialize directories/tables and stores receipts. Inspect source and previously captured evidence first; any future exercise should use an approved isolated store rather than pointing a diagnostic at an uncertain live path.

## 6. Stop at the implemented boundary

No rkyv archive is created or admitted in this fixture. The [derived-cache contract](../../rkyv-derived-cache-boundary.md) permits tagged rebuildable sidecars, but does not turn this roundtrip into a demonstrated mmap pipeline. Likewise, local fixture admission and receipt parsing do not establish release authority. A complete operational experiment needs the real caller's authority, consumer schema validation, and backend evidence in addition to this local trace.

## Sources

- [Handbook](../README.md)
- [Canonical architecture](../../architecture.md)
- [Schema Identity and Evolution](../../technical/envelopes/schema-identity-and-evolution.md)
- [rkyv derived-cache boundary](../../rkyv-derived-cache-boundary.md)
- [Roundtrip and migration fixtures](../../../src/typed/parts/storage/tests/m000/p000/body.rs)
- [Typed put and transaction implementation](../../../src/typed/parts/storage/p001/body.rs)
- [Typed get checks](../../../src/typed/parts/storage/p002/body.rs)
- [Schema inference](../../../src/typed/parts/storage/p009/body.rs)
