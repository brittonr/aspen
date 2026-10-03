# Storage, schema, and cache reference
Mode: Reference

Use this page while interpreting a captured entry, recipe, compatibility result, or archive manifest. Names below come from inspected source declarations; they are not a replacement schema specification or a promise that every caller enforces the governing contract. This is source-checked reference material, not executed runtime evidence. Return to the [Handbook](../README.md) for workflows and the [derived-cache technical companion](../../technical/storage/derived-cache-trust-model.md) for explanatory context.

## Identity and ownership map

| Identifier or artifact | Meaning | Implementation owner and limit |
| --- | --- | --- |
| `value_ref` | Canonical Preserves value identity | Typed storage uses the Preserves rail's canonical hash; not Rust or archive bytes |
| `schema_ref` on an inferred write | Hash of a value-class schema artifact | `inferred_schema_value`; class-level, not a field validator |
| `storage_ref` | Hash of the typed storage binding | Storage entry includes schema, payload, revision, and provenance-related references |
| `identity_ref` | Hash of `schema-identity-v1` | Schema identity module; differs from its embedded `schema_ref` |
| `structural_fingerprint` | Domain-separated normalized-shape identity | Schema identity module; equal fingerprint does not override unique identity |
| `manifest_ref` on a derived archive | Hash of canonical manifest metadata | Cache module; does not make archive layout canonical |
| `archive_byte_digest` | Identity of observed archive bytes for comparison | Shell must measure bytes; admission consumes the supplied observation |

Canonical Preserves and BLAKE3 remain the identity boundary. A backend filename locates data; a receipt records a result; neither grants authority to use the data.

## Typed-storage inputs and outputs

| API or type | Real fields or behavior | Owner / review point |
| --- | --- | --- |
| `PutInput` | `namespace`, `key`, optional `schema_ref`, `value`, `producer_ref`, `policy_refs`, `evidence_refs`, `admission` | Storage put validates a declared schema against inferred class |
| `Admission` | `actor_ref`, `capability_ref`, `policy_ref`, `resource_refs`, `evidence_refs` | Caller supplies context; `local_fixture` generates fixture references only |
| `Put` | `storage_ref`, `typed_ref_value`, `schema_ref`, `value_ref`, `receipt_value` | Keep value and binding identities separately |
| `Get` | `storage_ref`, `typed_ref`, `value`, `receipt_value` | Read path checks binding and canonical content hash |
| `SchemaCompatibilityGetInput` | Root, namespace/key, expected schema, compatibility value, admission | Compatibility path does not itself transform the value |
| `Migrate` | Old/new storage refs, old/new value refs, recipe ref, typed binding, receipt | Identifies a local migration result, not global cutover |
| `Payload` | `Inline { length }` or `ContentRef { manifest_ref, length }` | Describes canonical payload materialization |

`EntryRef` additionally exposes schema identity mode, producer and consumer refs, handler profile, policy/capability/retention/provenance/evidence/decoder refs, revision, actor, capability, effect handle, and checks. Their presence supports traceability, not automatic validation of every external artifact they name.

## Local artifacts and bounds

| Location or constant | Purpose | Operational qualification |
| --- | --- | --- |
| `typed-storage.redb` | Redb index file | Opening through these helpers may create tables |
| `typed_storage_records_v1` | Current binding by namespace/key hash | Distinct from reference-addressed historical entries |
| `typed_storage_refs_v1` | Binding by storage reference | Not a permission registry |
| `typed_storage_inline_values_v1` | Canonical inline bytes by value reference | Read checks recorded length |
| `typed_storage_receipts_v1` | Canonical receipt bytes by receipt reference | Reads and denials may add receipts |
| `chunks` | Chunk-store root below typed-storage root | Large-value payload effects precede index commit |
| `INLINE_VALUE_LIMIT = 4096` | Inline cutoff in canonical bytes | Not text-character count or Rust object size |

The put transaction combines index entries, inline bytes when applicable, and its receipt. That boundary does not establish atomic rollback of prior chunk writes. These are implementation facts from the storage helpers, not a general durability contract.

## Compatibility decisions

The decision order is policy denial, exact artifact match, admitted directional alias, structural match, brand match, migration available, then mismatch requiring migration. The corresponding strings are `denied-by-policy`, `exact-artifact-match`, `admitted-alias`, `structural-match`, `brand-match`, `migration-available`, and `mismatch-requires-migration`.

The storage compatibility helper accepts the five positive decisions, including `migration-available`, after matching expected/actual schema refs. It does not execute a recipe. Alias scopes recognized by the producer include `storage`, `effect`, `protocol`, `policy`, and `global-local-fixture`; the shared consumer does not independently recover and recheck the alias's scope. Keep that source-review limit visible when reviewing cross-domain consumers.

## Derived archive manifest and observations

| Field group | Manifest fields | Observation owner |
| --- | --- | --- |
| Classification | `cache_purpose`, `artifact_kind`, `profile_version` | Producer; admission supports `rkyv-derived-cache-v1` |
| Producer | `producer_tool_ref`, `producer_version` | Producer provenance, not trust proof |
| Sources | `source_digests` of `source_ref` plus `blake3_digest` | Shell supplies fresh `current_sources` |
| Bytes | `archive_byte_digest` | Shell supplies `observed_archive_digest` |
| Validation | `validation_required`, optional `validation_receipt_ref` | Shell supplies receipt observation and `validation_passed` |
| Lifecycle | Optional `rebuild_capability`, `retention_class`, `identity_claim` | Caller separately supplies `caller_allows_rebuild` |

Source lists must be nonempty, contain no duplicate refs, and have at most 64 entries. Retention classes are `ephemeral-cache` and `replay-snapshot`. The accepted admission identity claim is `derived-sidecar`. A manifest may be constructed with an unsupported profile or different nonempty claim, then denied at admission; construction is not admission.

## Worked interpretation

A schema-rename migration can produce equal old/new value refs and different storage refs because the binding changes while the canonical value does not. That is consistent with the implemented identity/schema-rename transforms, both of which clone the value. Conversely, a changed archive digest says nothing by itself about a changed canonical value. Compare each identifier in its own domain rather than treating every changed hash as data loss or every equal hash as compatibility.

## Sources

- [Handbook](../README.md)
- [rkyv governing boundary](../../rkyv-derived-cache-boundary.md)
- [Derived-cache trust model](../../technical/storage/derived-cache-trust-model.md)
- [Storage declarations](../../../src/typed/parts/storage/p000/body.rs)
- [Storage transaction](../../../src/typed/parts/storage/p001/body.rs)
- [Payload and index helpers](../../../src/typed/parts/storage/p005/body.rs)
- [Compatibility decisions](../../../src/schema/parts/identity/p001/body.rs)
- [Archive fields](../../../src/eval/parts/cache/p007/body.rs)
- [Archive validation and decisions](../../../src/eval/parts/cache/p008/body.rs)
