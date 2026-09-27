# Reviewing a data migration plan
Mode: Review checklist

Use this checklist before approving a typed-storage migration experiment or accepting its evidence. Each checked item needs a concrete artifact, inspected call path, or recorded observation; a receipt label alone is insufficient. This page is source-checked and no migration or validation command was executed for this handbook batch. It supplements the [Handbook](../README.md) and [Schema Identity and Evolution](../../technical/envelopes/schema-identity-and-evolution.md), not the governing contracts.

## Establish exactly which operation is proposed

- [ ] Does the plan distinguish a pure plan-identity calculation, compatibility admission, and actual storage mutation? The [Schema Migration Core pilot](../../schema-migration-core-pilot.md) computes deterministic plan identity and an unknown-fixture negative case; it executes no recipe, storage, transaction, or runtime effect. Do not accept pilot success as migration completion.
- [ ] Does it name the actual entry point and effect owner? `migrate_value` performs local storage work. `get_value_with_schema_compatibility` does not transform data. `get_value_with_migration` can execute a migration after a failed initial read. Attach the relevant source path, not only a UI label such as “upgrade.”
- [ ] Are namespace, key, current storage reference, current value reference, source schema, and desired target schema captured? These identifiers answer different questions. Canonical identity must remain Preserves+BLAKE3, never an archive digest or Rust layout.

## Check recipe claims against available execution

- [ ] Is the recipe a parsed `storage-migration-recipe-v1` with source/target schema refs, transformer ref/kind, mode, effect manifest, handler profile, policy, provenance, source-gate, test-evidence, rollback, lineage, evidence, and required checks? Use the [recipe parser](../../../src/typed/parts/storage/p004/body.rs) to identify what is actually validated.
- [ ] Does the proposed transformation fit the implemented transform? `identity` and `schema-rename` both clone the old value. The inspected path does not execute an arbitrary referenced program. A request to add fields, translate units, or repair malformed content is not implemented by changing the target schema reference.
- [ ] Are evidence references backed by inspectable artifacts and their real owners? The recipe builder synthesizes default source-gate, provenance, test-evidence, rollback, and lineage references. Their existence and check labels do not prove that tests, rollback, or external policy execution occurred.
- [ ] Is target conformance independently demonstrated at the required semantic level? The migration path validates canonical content and executable-authority restrictions, then binds the recipe's target schema. Do not treat its output-validation phase label as proof a general target-schema validator ran.

## Review authority and read behavior

- [ ] Is admission obtained from the real calling context rather than `Admission::local_fixture`? Stored values, recipes, content references, and receipts do not mint load or migration authority.
- [ ] If compatibility avoids a migration, does the reviewed producer bind expected/actual refs, identity modes, directional alias, and policy decision? The shared compatibility consumer accepts `migration-available`; that is not transformation evidence. It also does not independently re-establish alias scope from its referenced artifact.
- [ ] For lazy reads, has the reviewer inspected the error branch? The [lazy wrapper](../../../src/typed/parts/storage/p008/body.rs) attempts its recipe after any initial get error, subject to recipe/target/mode checks, not only a typed schema-mismatch category. This source-review observation requires explicit failure analysis; it is not a reason to retry an uncertain effect automatically.

## Identify persistence and recovery boundaries

- [ ] Does the plan distinguish chunk writes from the Redb index transaction? The [migration implementation](../../../src/typed/parts/storage/p003/body.rs) stores the payload before `persist_entry`. Index entry, inline data when applicable, and receipt commit together there; this is not proof of a transaction across the entire store.
- [ ] Is the recovery plan concrete about uncertain completion? Capture old/new storage refs, old/new value refs, recipe ref, revision, and receipt. Require evidence of current state before re-execution. Do not make deletion of state or unconditional retry the recovery strategy.
- [ ] Is rollback execution actually available and demonstrated for this operation? A `rollback_ref` and generated phase receipts are not an implemented rollback protocol. State the missing execution prerequisite rather than claiming recoverability from metadata alone.
- [ ] Is concurrent publication behavior accounted for by the owning integration? A local fixture roundtrip does not establish isolation against concurrent writers, global cutover, exactly-once migration, or crash recovery.

## Review derived caches separately

- [ ] Are affected sidecars identified by canonical source references rather than treated as authoritative data? Follow the [derived-cache boundary](../../rkyv-derived-cache-boundary.md).
- [ ] Does any proposed rebuild have both caller permission and its capability, with fresh byte/source/validation observations? A pure `rebuild` result does not perform shell I/O. Required validation failures or identity overclaims must remain denials, not be relabeled ordinary misses.

## Worked review outcome

The [explicit migration fixture](../../../src/typed/parts/storage/tests/m000/p000/body.rs) stores `<profile "alice" 7>`, constructs a `schema-rename` recipe, observes a target-schema read denial before migration, and asserts equal old/new value hashes afterward. This supports reviewing a local binding change with unchanged canonical data. It does not support a claim that an age field was converted or a production rollback was exercised.

Accept an experiment only for the behavior its evidence covers: correct source binding, supported transform, expected new binding, preserved canonical value where intended, and attributable receipts. Hold a broader release decision if target semantic validation, real authorization, recovery, or live integration evidence is missing. The checked-in assertions were inspected, not run here.

## Sources

- [Handbook](../README.md)
- [Schema Migration Core pilot](../../schema-migration-core-pilot.md)
- [Schema Identity and Evolution](../../technical/envelopes/schema-identity-and-evolution.md)
- [rkyv derived-cache boundary](../../rkyv-derived-cache-boundary.md)
- [Recipe construction and parsing](../../../src/typed/parts/storage/p004/body.rs)
- [Migration and phase evidence](../../../src/typed/parts/storage/p003/body.rs)
- [Supported transforms](../../../src/typed/parts/storage/p005/body.rs)
- [Lazy migration wrapper](../../../src/typed/parts/storage/p008/body.rs)
- [Migration fixtures](../../../src/typed/parts/storage/tests/m000/p000/body.rs)
