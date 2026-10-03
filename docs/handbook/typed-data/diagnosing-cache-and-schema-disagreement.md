# Diagnosing cache and schema disagreement
Mode: Troubleshooting

A valid archive can describe stale sources, and an intact canonical value can carry a schema the consumer must not interpret. Diagnose those boundaries separately. This guide is source-checked, not a report of reproduced failures or executed commands. It complements the [Handbook](../README.md) and [Derived-cache trust model](../../technical/storage/derived-cache-trust-model.md); the [rkyv boundary](../../rkyv-derived-cache-boundary.md) remains governing.

## Preserve evidence before attempting recovery

Collect the stored binding, canonical value reference, expected schema reference, supplied compatibility artifact if any, archive manifest, current source observations, observed archive digest, validation result, and receipt reference. Record which caller measured each observation. A pure admission function cannot establish that a caller measured the intended bytes.

Prefer existing evidence and source inspection first. Typed-storage get, verify, and receipt helpers may initialize state or store receipts. They are not guaranteed read-only probes. Do not delete state, replace canonical data with archive bytes, or invoke a lazy migration merely to see whether the error disappears. A future diagnostic exercise belongs in an approved isolated store with explicit effect ownership.

## Symptom: the archive validates, but admission says stale

**Discriminating evidence:** compare manifest `source_digests` with `current_sources`, as pairs of reference and digest. The implementation checks equal lengths and set equality, so reordering a valid source list is not staleness. A matching archive-byte digest and passing byte validation do not settle source freshness.

**Safe next action:** identify which canonical source changed and whether the producer selected the correct source set. A `rebuild` decision requires caller permission, a present rebuild capability, and exclusively rebuildable diagnostics. Treat it as a decision for the shell to act on, not proof a new archive exists.

**Stop condition:** any missing source provenance or unrelated denial diagnostic blocks using the old archive as a fallback. Rebuilding without knowing the intended sources can faithfully cache the wrong data.

## Symptom: stale archive unexpectedly produces deny, not rebuild

**Discriminating evidence:** retain the complete diagnostics list. The [decision helper](../../../src/eval/parts/cache/p008/body.rs) currently classifies text containing `stale` or `byte digest` as rebuildable. Required validation failure, required validation-receipt mismatch, unsupported profile, and canonical-identity overclaim do not qualify.

**Safe next action:** inspect the manifest requirement and exact observed validation evidence. Correct the observation or producer integration through its owner; do not suppress validation, rewrite the decision, or silently change the identity claim to proceed.

**Stop condition:** a combined stale-plus-validation failure is denied even if rebuild is permitted. A rebuildable cache concept is not blanket permission to repair every failed admission automatically.

## Symptom: the sidecar is admitted, but the typed read rejects the schema

**Discriminating evidence:** compare the consumer's expected schema with the stored binding, not with the archive profile. Archive admission does not supply schema compatibility. For unequal refs, inspect the compatibility artifact's ordered expected/actual pair and decision.

**Safe next action:** follow [Checking schema compatibility before a read](checking-schema-compatibility-before-read.md). Determine whether this is structural compatibility, a reviewed directional alias, or a real migration requirement. Keep field-level application checks separate from storage's inferred Preserves class.

**Stop condition:** matching bytes, matching Rust types, or a valid archive receipt cannot override a schema mismatch. Do not remove the expected schema from the request to turn a denial into success.

## Symptom: compatibility says migration available, but the value is unchanged

**Discriminating evidence:** inspect the actual entry point. `get_value_with_schema_compatibility` accepts the positive `migration-available` decision and reads existing canonical data; it does not call the migration transform. The separate `get_value_with_migration` path can execute a recipe after an initial get error.

**Safe next action:** distinguish a permitted interpretation from required transformed data. If transformation is necessary, review the recipe, target schema, transformer, and effect boundary before any execution. The current typed-storage transforms `identity` and `schema-rename` both clone the original value.

**Stop condition:** a migration reference or compatibility receipt is insufficient evidence that data was transformed. The shared migration-core pilot also executes no storage or recipe effects, as its [governing note](../../schema-migration-core-pilot.md) states.

## Symptom: the read fails with a payload or content-integrity error

**Discriminating evidence:** separate missing inline data, recorded-length mismatch, chunk-store failure, canonical parse failure, and final value-hash mismatch. The [payload reader](../../../src/typed/parts/storage/p005/body.rs) checks lengths; the [get path](../../../src/typed/parts/storage/p002/body.rs) recomputes canonical value identity after parsing. Not every early error necessarily produces the same denial receipt.

**Safe next action:** preserve the binding and backend evidence, then involve the owning storage/recovery workflow. Rebuilding a derived archive cannot repair missing authoritative canonical data. Avoid unconditional retries after an uncertain write or migration outcome; first establish the indexed revision and captured operation evidence through an approved diagnostic path.

**Stop condition:** never use a passing sidecar validation as a substitute for failed canonical content integrity.

## Worked triage case

Suppose a replay-index manifest records source observation A, while the current observation is B. Its archive digest still matches. Staleness alone may yield `rebuild` when both permissions are present. Add a required validation failure and the decision becomes `deny`. Separately, the stored value may still read successfully under its exact schema: these outcomes are not contradictory because cache usability, canonical integrity, and consumer interpretation are independent checks. This is an illustrative source-derived case, not terminal output or a fabricated digest vector.

## Sources

- [Handbook](../README.md)
- [rkyv derived-cache boundary](../../rkyv-derived-cache-boundary.md)
- [Derived-cache trust model](../../technical/storage/derived-cache-trust-model.md)
- [Migration-core pilot limits](../../schema-migration-core-pilot.md)
- [Cache admission diagnostics](../../../src/eval/parts/cache/p008/body.rs)
- [Typed get checks](../../../src/typed/parts/storage/p002/body.rs)
- [Payload reads and transforms](../../../src/typed/parts/storage/p005/body.rs)
- [Lazy migration control flow](../../../src/typed/parts/storage/p008/body.rs)
