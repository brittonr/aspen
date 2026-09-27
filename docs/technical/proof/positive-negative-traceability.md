# Positive Negative Traceability

Traceability connects a requirement to evidence of permitted behavior and evidence that prohibited behavior is rejected. It is more precise than a list of passing test names: coverage purpose, subject identity, artifact references, and exemptions all matter. This article builds on the [proof workflow](../../proof-workflow.md), focusing on the coverage manifest rather than the mechanics of individual subprocesses. Related articles are collected in the [Technical companion](../README.md).

## Coverage is a relation, not a test count

`TraceabilityInput` carries requirement descriptions, coverage entries, and an explicit `require_receipt_backed` policy. A `CoverageInput` separates positive and negative vectors and can carry an exemption. The [entry builder](../../../src/testing/traceability/parts/p003/body.rs) treats changed requirements and requirements whose kind is `evidence` as requiring coverage. This is narrower than saying every requirement always needs a new execution in every scan.

Requirements are indexed by identity; duplicate requirement definitions and duplicate unmerged coverage entries are errors. Coverage referring to a requirement absent from the supplied requirement set becomes a stale entry rather than being silently discarded. That behavior matters during requirement renames: a valid old receipt is not automatically evidence for the new identifier. The receipt binding supplies identity, not a semantic migration rule between requirements.

The two coverage directions are independent. A positive example demonstrates an admitted behavior under its stated conditions. A negative example demonstrates a rejected behavior under its stated conditions. Neither direction is an exhaustive proof over all inputs. The [distributed-testing contract](../../distributed-testing.md) uses the same split for simulation fixtures, CI profile evidence, and unavailable platform cases; a skipped VM environment is not positive platform evidence merely because simulation coverage exists.

## Admission and precedence

The [evidence validators](../../../src/testing/traceability/parts/p008/body.rs) inspect target presence, command presence, artifact references, receipt references, source labels, and expected decisions. A non-compatibility item needs a receipt reference. Duplicate receipt references are detected within each validated evidence list. This is not a global uniqueness claim over every receipt used by the repository.

When `require_receipt_backed` is true, items labeled `compatibility` receive stale diagnostics. Under a permissive policy those items remain visibly labeled in the summary; the existence of a compatibility tuple is not retroactively upgraded into a canonical verification observation. `artifact_present` and `target_exists` are supplied or derived values at this layer, not proof that the pure manifest builder performed filesystem inspection.

Status selection is ordered. Stale diagnostics take precedence over an exemption; a recognized exemption takes precedence over missing coverage; missing positive precedes missing negative; otherwise a required or populated entry can be covered. Thus an entry missing both coverage directions initially appears as missing positive, not as two independent statuses. The negative gap becomes relevant after positive coverage is supplied. The summary retains separate missing-positive and missing-negative groups, and the overall traceability decision denies when either group or the stale-reference group is nonempty.

Exemption classes are explicit: `documentation-only`, `operator-guidance`, and `non-executable`. Their evidence text is validated, and an unsupported exemption class becomes stale evidence. An exemption explains why executable coverage is inapplicable; it is not a passing run and does not transform unrelated stale evidence into acceptable coverage.

## Worked migration scenario

Consider an illustrative requirement whose admission behavior changes. A developer supplies a passing receipt for the current target and two copies of an old negative receipt, believing the duplicate strengthens confidence. Receipt derivation groups both negative entries under the requirement. During evidence validation, the duplicate receipt reference produces a stale diagnostic. Adding a documentation-only exemption would not hide that stale state because stale status wins.

After removing the duplicate, suppose the target has moved and the shell reports `target_exists = false`. The entry remains stale. Updating only a rendered summary would change no canonical evidence. The appropriate review asks whether the moved target preserves the same requirement and whether a fresh verification observation is needed; the manifest cannot answer that semantic question from a path string alone.

Finally, suppose both current receipts are present. Review still follows the negative artifact to confirm the intended denial. The [receipt conversion](../../../src/testing/traceability/parts/p002/body.rs) carries the receipt decision into coverage, but not its diagnostics. As explained in the [verification receipt contract](verification-run-receipt-contract.md), this is a scoped discrepancy with the governing fail-closed wording: an invalid negative observation can also have decision `deny`. A covered manifest therefore does not remove the need to inspect negative-run validity.

## Review guidance and limits

The existing tests include strict-policy rejection of raw tuples and receipt-backed positive/negative derivation. Suggested targeted checks are `cargo test --lib receipt_backed_policy_denies_raw_compatibility_tuples` and `cargo test --lib verification_run_receipts_derive_receipt_backed_coverage`; neither is claimed as executed here.

A useful review starts with the requirement set, then examines purpose-separated evidence, then exemptions and stale entries, and only afterward reads the summary. Keep the canonical manifest distinct from terminal output and retain the policy that governed its construction. Traceability demonstrates the supplied relationship between requirements and evidence. It does not grant execution authority, establish production readiness, or prove that every behavior implied by a natural-language requirement has been tested.

## Sources

- [Proof workflow](../../proof-workflow.md)
- [Distributed testing evidence](../../distributed-testing.md)
- [Traceability entry construction](../../../src/testing/traceability/parts/p003/body.rs)
- [Evidence and exemption validation](../../../src/testing/traceability/parts/p008/body.rs)
- [Summary decisions and canonical values](../../../src/testing/traceability/parts/p004/body.rs)
- [Receipt conversion and regression tests](../../../src/testing/traceability/parts/tests/p001/body.rs)
- [Technical companion](../README.md)
