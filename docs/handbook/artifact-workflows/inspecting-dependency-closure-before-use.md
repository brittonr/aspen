# Inspecting dependency closure before use

Mode: How-to

## Goal and prerequisites

The goal is to assemble a bounded local availability observation for an exact artifact before handing it to a consumer. This is not an import authorization recipe. You need an already available `molten` binary, an identified registry, the exact artifact reference from retained evidence, and permission to inspect that registry. Commands below are source-checked, not executed for this guide. No working development-shell or runtime result is implied.

Use the [Handbook](../README.md) for navigation. The [content-reference companion](../../technical/envelopes/content-references-versus-inline-values.md) explains why valid reference syntax, present bytes, and authorization are separate conclusions.

## 1. Decide which state may be touched

Prefer an isolated inspection registry prepared through an approved state-copy or materialization procedure. Do not improvise a filesystem copy of an active database. Preserve the original state and its provenance through the registry owner's procedure.

These commands are not uniformly read-only. Even loading helpers call `ensure_index_tables`, which creates directories, opens a database with creation enabled, and ensures tables. Closure inspection additionally stores a receipt. Verify the selected registry is the intended existing directory before proceeding; do not point diagnostics at a misspelled path and mistake a newly created empty registry for evidence of lost artifacts.

The inspected [registry initialization](../../../src/artifacts/parts/mod/p013/body.rs) and [closure receipt persistence](../../../src/artifacts/parts/mod/p008/body.rs) support these cautions. If you cannot permit those writes, stop at source and retained-evidence review until the state owner provides a suitable inspection surface.

## 2. Resolve the exact subject before querying

A display name is insufficient. Obtain the exact artifact reference that the consuming request, job, or unit has pinned. If investigating a pointer change, retain both the observed pointer and the prior exact reference; do not repeatedly resolve a moving name during the same unit.

For an approved registry, inspect the artifact record and direct dependency list. Set `REGISTRY` to the existing inspection directory and `ARTIFACT_REF` to the evidence-backed reference; the guards intentionally supply no sample hash. The spelling is supported by the [root command declaration](../../../src/main/root/parts/command/p000/body.rs), [artifact command declaration](../../../src/cli/core/artifact/command.rs), and [view/deps operations](../../../src/cli/core/artifact/ops.rs). Source-checked; not executed:

```sh
: "${REGISTRY:?Set the approved existing inspection registry}"
: "${ARTIFACT_REF:?Set the exact artifact ref from retained evidence}"
test -d "$REGISTRY" && molten test artifact view "$ARTIFACT_REF" --registry "$REGISTRY"
test -d "$REGISTRY" && molten test artifact deps "$ARTIFACT_REF" --registry "$REGISTRY"
```

`view` without `--payload` prints the artifact record. `deps` reads that record and lists only `dependency_refs`. Retain schema, effect-manifest, policy, and evidence refs separately: their presence in the record is not proof that their targets are loaded or admitted.

## 3. Request the transitive observation

Select a new receipt destination that will not overwrite previous evidence. `RECEIPT_OUT` must name a fresh file under an approved output directory. The [closure declaration](../../../src/cli/core/artifact/command.rs), [operation](../../../src/cli/core/artifact/ops.rs), and [receipt writer](../../../src/cli/core/artifact/io.rs) support this command. Source-checked; not executed:

```sh
: "${REGISTRY:?Set the approved existing inspection registry}"
: "${ARTIFACT_REF:?Set the exact artifact ref from retained evidence}"
: "${RECEIPT_OUT:?Set a fresh closure receipt file path}"
test -d "$REGISTRY" && test ! -e "$RECEIPT_OUT" &&
  molten test artifact closure "$ARTIFACT_REF" --registry "$REGISTRY" --receipt-out "$RECEIPT_OUT"
```

The destination guard prevents an ordinary existing-path overwrite, not races with another writer. Use a private output directory. The command prints available closure refs, reports missing refs on stderr, and writes the canonical receipt as Preserves text. Inspect its decision and diagnostics; do not interpret process success as closure completeness. The operation returns success after reporting a nonempty missing set.

## 4. Decide whether the observation is sufficient

If any reference is missing, stop before use. Give the exact missing set to the receiver-owned fetch/admission workflow. Do not accept unrelated sender-pushed extras as permission to import. If traversal exceeds a bound, preserve the error rather than treating a partial list as a complete closure.

If no reference is missing, distinguish registry availability from integrity. The [traversal](../../../src/artifacts/parts/mod/p011/body.rs) consults artifact existence and derived dependency entries; it does not independently decode and hash every artifact or payload. A higher-assurance consumer must load and verify the exact members through its owning boundary. The direct artifact reader compares canonical record identity to the requested ref; payload loading performs a separate path.

## Worked decision: dependent present, schema metadata unresolved

The [dependency fixture](../../../src/artifacts/parts/mod/tests/m000/p001/body.rs) installs `base`, then `dependent` importing `base`. Closure includes both. Its helper also supplies schema, policy, and evidence refs without installing all those referenced objects. Thus a complete import closure must not be relabelled “all evidence verified.” Carry the closure observation forward, but require the consuming policy/schema/effect loaders to discharge their separate obligations.

For late binding, one immutable snapshot and exactly one matching supplied closure are required by [unit resolution](../../../crates/molten-core/src/live_binding/binding.rs). That pure helper normalizes supplied dependency IDs; it does not fetch or prove their contents. Stop before execution or cutover if verification, compatibility, migration, or current admission evidence is absent.

## Sources

- [Handbook](../README.md)
- [Content-reference companion](../../technical/envelopes/content-references-versus-inline-values.md)
- [Reference execution admission](../../unison-reference-execution.md)
- [Live binding contract](../../live-artifact-binding-and-semantic-effects.md)
- [Artifact CLI commands](../../../src/cli/core/artifact/command.rs)
- [Artifact CLI operations](../../../src/cli/core/artifact/ops.rs)
- [Registry traversal](../../../src/artifacts/parts/mod/p011/body.rs)
- [Registry initialization](../../../src/artifacts/parts/mod/p013/body.rs)
