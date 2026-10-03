# Diagnosing missing or incompatible artifacts

Mode: Troubleshooting

Treat the first failed boundary as the diagnostic subject. A missing local registry entry, corrupt payload, unresolved name, incompatible semantic operation, and stale binding publication are different failures. Reinstalling or rebuilding indiscriminately can obscure the evidence without fixing the relevant boundary.

This guide is source-checked, not a reproduced incident or executed test report. Use the [Handbook](../README.md) for navigation and the [closure how-to](inspecting-dependency-closure-before-use.md) for guarded inspection commands. The [content-reference companion](../../technical/envelopes/content-references-versus-inline-values.md) explains why a matching reference does not prove availability or authority.

## Symptom: a reference is rejected before loading

**Discriminating evidence:** distinguish malformed reference syntax from a well-formed reference that is absent. Registry validation requires canonical `blake3:` content-ref spelling. The [malformed-reference fixture](../../../src/artifacts/parts/mod/tests/m000/p000/body.rs) separately covers a short schema ref, uppercase characters in a manifest ref, and a valid-shaped but missing artifact. These are distinct input classes, not interchangeable “not found” cases.

**Safe next action:** recover the exact reference from the original canonical artifact or retained request. Check the field's owner: an artifact ref is not a pointer name, path, handler alias, or raw source digest. Do not normalize a suspicious reference into acceptance without recovering the authoritative input representation.

**Stop condition:** if exact bytes or reference provenance cannot be established, stop before import. Inventing a well-shaped hash repairs neither identity nor availability.

## Symptom: install returns a denial but the process succeeds

**Discriminating evidence:** examine `decision`, `missing_dependencies`, and the install receipt. The [installer](../../../src/artifacts/parts/mod/p006/body.rs) can return a successful Rust result containing `deny`; the [CLI operation](../../../src/cli/core/artifact/ops.rs) prints it and returns success. With `--artifact-out`, it can also emit the proposed artifact record even though the registry did not store that artifact.

**Safe next action:** preserve the denial and exact missing set. The [commit helper](../../../src/artifacts/parts/mod/p001/body.rs) stores the receipt but omits the artifact when dependencies are missing. Ask the receiver-owned admission workflow to evaluate the missing objects; do not mistake exported proposed metadata for successful installation.

**Stop condition:** do not run the consumer until the exact required dependencies have been obtained, verified, and admitted. A repeat install without changed evidence is not a diagnosis.

## Symptom: closure looks complete, but loading fails

**Discriminating evidence:** the [closure traversal](../../../src/artifacts/parts/mod/p011/body.rs) checks artifact-table presence and reads derived dependency entries. It does not remeasure every artifact or load each payload. By contrast, `read_artifact_with_root` decodes the stored canonical record and compares its hash with the requested reference. `read_payload_with_root` follows an additional inline or chunk path.

**Safe next action:** preserve the failing reference and exact error boundary. An artifact-record mismatch, missing inline payload, payload mismatch, or chunk-read failure requires its own state/integrity investigation. Keep the registry and associated chunk state available for the owner; do not delete either to make the error disappear.

**Stop condition:** a complete indexed closure cannot override a failed content read. If a derived-index discrepancy is suspected, review the source and maintenance procedure before any mutation. `index-rebuild` is an explicit write operation, not a cure for corrupt source records.

## Worked failure: wrong canonical record under a correct-looking key

The [tampered-materialization fixture](../../../src/artifacts/parts/mod/tests/m000/p000/body.rs) installs two different artifacts, then deliberately stores the second artifact's canonical bytes under the first artifact's key. Reading the first ref must report an artifact registry content-hash mismatch. Both records may individually be canonical; the failure is the relationship between the requested ref and returned bytes.

The useful response is to retain the key, observed record, and mismatch evidence for integrity review. Renaming the key, suppressing the mismatch, or substituting the second ref changes the requested subject. This is an inspected test scenario, not a claim that corruption was reproduced in a running deployment for this guide.

## Symptom: a name resolves differently or ambiguously

**Discriminating evidence:** compare view kind, scope, candidate views, and supplied stale-view refs. The [name-view fixture](../../../src/artifacts/parts/mod/tests/m000/p001/body.rs) distinguishes scoped resolution from ambiguous unscoped candidates and stale candidates. A display label alone cannot identify which exact artifact old work pinned.

**Safe next action:** retain the exact resolution receipt and target used by the affected unit. Review scope and snapshot evidence instead of repeatedly resolving the current pointer. Old work retaining an older exact target after cutover is required behavior, not necessarily a stale-cache fault.

**Stop condition:** do not replace a pinned unit's target merely because a name now points elsewhere. A new resolution belongs to a new explicit unit or declared nested operation.

## Symptom: content loads, but binding or handler use is denied

**Discriminating evidence:** inspect the failing [product gate](../../../crates/molten-core/src/live_binding/binding.rs), missing/duplicate closure error, implicit nested lookup, or [semantic compatibility context](../../../crates/molten-core/src/live_binding/semantic.rs). Target availability does not discharge migration, policy, capability, provenance, resource, or lifecycle obligations. Exact handler identities must agree; matching names or shapes are insufficient.

**Safe next action:** collect evidence for the denied gate and preserve the old binding. For stale compare-and-swap publication, reload current state through the owning shell and obtain a new checked plan rather than reapplying an uncertain effect. For semantic mismatch, require exact equality or explicitly directional, context-bound compatibility plus current admission.

**Stop condition:** neither a pure plan nor a receipt authorizes publication. Follow the [live-binding contract](../../live-artifact-binding-and-semantic-effects.md); no availability workaround justifies bypassing a gate or deleting pinned state.

## Sources

- [Handbook](../README.md)
- [Content-reference companion](../../technical/envelopes/content-references-versus-inline-values.md)
- [Reference execution contract](../../unison-reference-execution.md)
- [Live binding contract](../../live-artifact-binding-and-semantic-effects.md)
- [Malformed and tampered artifact fixtures](../../../src/artifacts/parts/mod/tests/m000/p000/body.rs)
- [Name and closure fixtures](../../../src/artifacts/parts/mod/tests/m000/p001/body.rs)
- [Artifact loading and commit](../../../src/artifacts/parts/mod/p001/body.rs)
- [CLI behavior](../../../src/cli/core/artifact/ops.rs)
