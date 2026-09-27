# Verification Run Receipt Contract

A verification-run receipt binds an explicit verification observation to a requirement and coverage purpose. It is not the test process itself, and it is not authority to perform the operation being tested. This article assumes familiarity with the [proof workflow](../../proof-workflow.md) and distinguishes receipt construction, receipt parsing, and downstream coverage admission. See the [Technical companion](../README.md) for related topics.

## The observation being committed

`VerificationRunInput` supplies requirement identity, coverage kind, target, argument vector, profile reference, toolchain references, exit status, stdout and stderr references, and produced artifact references. The corresponding canonical value is tagged `verification-run-receipt-v1`. Its serialization preserves `argv` as a sequence rather than reducing command identity to a shell command string. The profile and toolchain references describe the supplied execution context; they do not cause the builder to obtain that context from the machine.

The [receipt implementation](../../../src/testing/traceability/parts/p004/body.rs) validates text, coverage kind, bounded lists, and content-reference syntax. It derives diagnostics from two principal observations: whether exit status agrees with coverage purpose, and whether a produced artifact reference was supplied. Positive coverage expects status zero. Negative coverage expects nonzero status. A missing artifact produces `missing-produced-artifact-ref` even when exit status otherwise agrees.

This is a deterministic in-memory operation over supplied values. Creating a record that says a command ran is different from executing that command, collecting its artifacts, and establishing that those artifacts belong to the claimed run. The shell and the evidence-producing workflow remain responsible for those observations. Hashing the receipt binds the supplied representation; it cannot independently make false observations true.

## Two different meanings of denial

The important subtlety is that `deny` has more than one evidentiary meaning. An expected negative execution is represented by denial, but an invalid positive or negative observation can also produce denial. For example, a negative run that exits zero receives the diagnostic `negative-run-did-not-deny`; its receipt decision is still `deny`. Consequently, checking only that a negative receipt contains that decision does not distinguish a successful negative control from a failed negative experiment.

The [parser](../../../src/testing/traceability/parts/p002/body.rs) checks record shape, schema, fields, reference syntax, and consistency of the supplied decision with coverage kind, exit status, and diagnostics. It recomputes `receipt_ref` using the canonical value. It does not execute the recorded argument vector, fetch output bytes, or infer why a command exited nonzero. A compiler error, unavailable executable, and intended admission rejection can all produce a nonzero process status; their semantic difference belongs in the underlying test and canonical artifacts.

There is also a source boundary worth retaining explicitly. The governing workflow says wrong exit status keeps traceability fail-closed. In the inspected implementation, receipt-to-coverage conversion copies the decision but not the diagnostics into `VerificationEvidence`; downstream decision validation compares that decision with the expected coverage decision. The article therefore does not claim that this conversion independently rejects every invalid negative observation. Review of exit status, diagnostics, and the expected-deny artifact remains necessary; the documentation statement is broader than the visible check at that conversion boundary.

## Worked negative-control reasoning

Consider an illustrative stale-evidence test. The intended command attempts admission using an obsolete subject reference, expects denial, and emits a subsystem artifact recording that denial before mutation. Its verification input uses negative coverage, the observed nonzero status, and references to that artifact and the captured outputs.

A reviewer can separate three questions:

1. Did the observation match the verification-run contract? Examine coverage kind, exit status, diagnostics, and artifact references.
2. Did the subsystem reject the intended stale-evidence case? Inspect the produced artifact rather than inferring the reason from nonzero status.
3. Did rejection precede mutation? Inspect unchanged state references or the no-mutation evidence required by the [proof workflow](../../proof-workflow.md).

If the executable instead fails to start, a nonzero status answers neither the second nor third question. If the command unexpectedly succeeds, `deny` on the verification receipt is a report of a bad negative run, not proof of subsystem denial. These distinctions prevent process-level observations from being silently promoted into domain-level evidence.

## Verification and review guidance

The [existing receipt tests](../../../src/testing/traceability/parts/tests/p001/body.rs) cover receipt-backed derivation, compatibility-only rejection under the strict policy, and diagnostics for a negative run with the wrong exit status. A focused suggested check is `cargo test --lib verification_run_receipts_derive_receipt_backed_coverage`; another is `cargo test --lib verification_receipt_denies_wrong_exit_for_negative_coverage`. These are reproduction suggestions, not executions reported by this article.

Review the canonical receipt before rendered summaries. Preserve argument boundaries, relate profile and toolchain references to the actual execution, and follow artifact references to the intended semantic outcome. Logs can explain failures, but their presence is not a substitute for the receipt or the subsystem claim. This contract supplies reproducible evidence structure, not a formal proof of program correctness, authenticated provenance of arbitrary supplied bytes, or release authorization.

## Sources

- [Proof workflow](../../proof-workflow.md)
- [Distributed testing evidence](../../distributed-testing.md)
- [Receipt validation and serialization](../../../src/testing/traceability/parts/p004/body.rs)
- [Receipt parsing and coverage conversion](../../../src/testing/traceability/parts/p002/body.rs)
- [Receipt regression tests](../../../src/testing/traceability/parts/tests/p001/body.rs)
- [Technical companion](../README.md)
