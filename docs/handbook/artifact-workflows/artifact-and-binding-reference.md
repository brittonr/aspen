# Artifact and binding reference

Mode: Reference

Use this page to identify the owner and meaning of a registry field or binding observation during an import review. It is a source-checked reference, not runtime verification. The [Handbook](../README.md) supplies practical navigation; the [canonical identity companion](../../technical/foundations/canonical-identity-model.md) supplies theory. Governing requirements remain in [reference execution](../../unison-reference-execution.md) and [live binding and semantic effects](../../live-artifact-binding-and-semantic-effects.md).

## Source and CLI entry points

`src/lib.rs` declares `objects` through the `artifacts/mod.rs` path; that file includes split implementation bodies. The CLI uses the `molten::artifacts` facade. `src/main.rs` maps `object_port` to `cli/core/artifact.rs` and re-exports it through `cli_artifact`; the root parser places these operations under `test artifact`, not a top-level `artifact` command.

The table gives command suffixes, not runnable recipes. Spelling and arguments come from the [root declaration](../../../src/main/root/parts/command/p000/body.rs), [artifact declaration](../../../src/cli/core/artifact/command.rs), and [operations](../../../src/cli/core/artifact/ops.rs). None was executed for this reference.

| Suffix after `molten test artifact` | Required subject/options | Result and caveat |
|---|---|---|
| `list` | `--registry`; optional `--kind` | Lists refs and kinds; not payload verification. |
| `view` | Positional artifact ref; `--registry`; optional `--payload` | Prints record or loaded payload. |
| `deps` | Positional artifact ref; `--registry` | Direct import refs from the verified artifact record. |
| `closure` | Positional artifact ref; `--registry`; optional `--receipt-out` | Indexed transitive availability and persisted receipt. Missing refs do not force CLI failure. |
| `impact` | Positional artifact ref; `--registry`; optional `--receipt-out` | Reverse dependency observation, not forward closure. |
| `name-show` | `--registry`, `--name`; optional `--kind` | Current pointer target; default pointer kind is `name`. |
| `install` | Positional Preserves payload file; `--registry` | Local registry mutation; CLI-generated evidence refs are not live credentials. |
| `index-rebuild` | `--registry`; optional `--receipt-out` | Mutates derived indexes. Not a first-line diagnostic. |

Opening a registry can initialize tables even for observational operations. Closure and impact persist receipts. Review the state owner and output destinations before treating these commands as harmless reads.

## Registry record and installation fields

The [record definitions](../../../src/artifacts/parts/mod/p000/body.rs) and [canonical constructor](../../../src/artifacts/parts/mod/p001/body.rs) own this vocabulary. Rust fields are API inputs/outputs; canonical Preserves, not Rust layout, defines identity.

| Field or artifact | Meaning | Do not infer |
|---|---|---|
| `artifact_ref` | Canonical hash of the parsed `artifact-v1` record | Permission to execute or publish |
| `kind`, `domain` | Artifact interpretation and kind-derived identity domain | Compilation or semantic correctness |
| `Inline { value_ref, length }` | Canonical payload reference and byte length | Payload is embedded in the artifact record |
| `ContentRef { manifest_ref, length }` | Chunk-manifest reference and payload length | Chunks are locally available |
| `dependency_refs` | Explicit artifact imports | Every referenced evidence/schema object |
| `schema_refs`, `effect_manifest_ref` | Separate schema/effect dependencies | Consumer admission has occurred |
| `policy_refs`, `evidence_refs` | Bound supporting references | Evidence is true, current, or applicable |
| Install `decision`, `missing_dependencies` | Local install outcome and missing direct imports | Returned record was committed on denial |
| `identity_receipt_ref` | Identity-check receipt reference | Authority or a publication receipt |
| `ArtifactClosure` | Roots, available refs, missing refs, closure hash, receipt | Independent whole-store integrity validation |

The inline cutoff is 4096 canonical bytes. Ref lists and traversal stacks are bounded at 4096; diagnostics at 256. These constants are not a promise that every 4096-member graph succeeds: combined receipt-reference counts and pending traversal entries are bounded too.

Dependency edge records distinguish `imports` for artifact dependencies, `validates-with` for schemas and policies, `invokes` for effects, and optional `documents` evidence edges. The ordinary closure traversal follows the stored import list, not this entire richer taxonomy.

## Binding and semantic fields

The [pure binding model](../../../crates/molten-core/src/live_binding/model.rs) and [implementation](../../../crates/molten-core/src/live_binding/binding.rs) own planning inputs. The shell supplies loaded facts and owns effects.

| Surface | Important fields/results | Owner and limit |
|---|---|---|
| `ProductGateFacts` | Target loaded/verified; product compatible; migration required/satisfied; authority, policy, provenance, resource, lifecycle admitted | Molten shell must substantiate each boolean. |
| `MoltenCutoverPlan` | `shared_plan`, `publication_authorized` | Pure planning returns publication authorization as false. |
| `UnitResolutionInput` | Boundary, request, optional snapshot, closures, nested-lookup declarations | Exactly one closure must match the resolved target. |
| `UnitResolution` | Shared resolution and normalized `pinned_dependencies` | Pins supplied facts; does not fetch or verify bytes. |
| `UnitBoundary` | Request, turn, callback pass, job, protocol session | Explicit unit scope, not repeated ambient lookup. |
| `RootInventoryInput` | Snapshot, generation, roots, edges, class/edge/attribution completeness | Retirement needs supplied complete observations. |
| `MoltenRetirementReport` | Decision and observation/retention/deletion flags | Retirement is not deletion authority. |

[Semantic matching](../../../crates/molten-core/src/live_binding/semantic.rs) requires exact operation identity by default. Directional compatibility additionally needs the exact compatibility artifact/context and admitted Molten policy, capability, and provenance facts. Replay-only compatibility cannot be promoted into live-host permission.

## Worked interpretation

Suppose a local install returns a proposed artifact and a denial receipt because one direct dependency is absent. The artifact reference remains meaningful as the proposed record's identity, but `commit_install` stores only the receipt in that branch. A later name or binding must not treat that returned reference as materialized content. If an isolated closure check later passes, the conclusion improves only to indexed import availability. Verified bytes, semantic compatibility, current admission, and atomic publication remain separate evidence entries.

## Sources

- [Handbook](../README.md)
- [Canonical identity companion](../../technical/foundations/canonical-identity-model.md)
- [Reference execution](../../unison-reference-execution.md)
- [Live binding contract](../../live-artifact-binding-and-semantic-effects.md)
- [Registry definitions and limits](../../../src/artifacts/parts/mod/p000/body.rs)
- [Canonical records and commit](../../../src/artifacts/parts/mod/p001/body.rs)
- [Dependency edge taxonomy](../../../src/artifacts/parts/mod/p002/body.rs)
- [Binding model](../../../crates/molten-core/src/live_binding/model.rs)
- [Canonical binding envelope labels](../../../src/parts/live_binding_adoption/p000/body.rs)
