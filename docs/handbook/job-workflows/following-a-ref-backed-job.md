# Following a ref-backed job

Mode: Walkthrough

This source-only walkthrough follows the checked-in `blob_ref_job_submission_worker_verifies_and_outputs_manifest` test from bytes to a result manifest. It is useful when reviewing a submission or deciding what evidence a local ref worker actually produces. It is not a tutorial for launching an arbitrary executable. No command or test was executed for this page. Return to the [Handbook](../README.md) for related workflows; the [content-reference companion](../../technical/envelopes/content-references-versus-inline-values.md) explains the identity model.

## 1. Identify the fixture's real inputs

The [fixture](../../../src/job/parts/dag/tests/m000/p000/body.rs) creates an isolated root with `chunks` and `ledger` children. It stores `b"echo"` as `job-executable` and `b"hello"` as `job-input`, using `put_bytes` and the fixed chunk-size constant. Each store operation supplies a manifest reference and total byte length.

The submission labels the first object `elf-executable`. That label does not make the four bytes an ELF program, and this fixture does not execute them as one. Keep three observations separate: bytes exist in the chunk store, their manifests identify them, and a handler determines how execution proceeds. A useful review note records both the manifest reference and the length returned by storage rather than copying an illustrative digest from the README.

## 2. Construct the canonical submission

The fixture supplies job ID `job-ref-worker`, a locally derived operation reference, and locally derived authority, policy, provenance, and effect references. Its selected handler is `local-echo-v1`; its output mode is `chunk-manifest`. There is one input, no schema refs, and no additional evidence refs.

[`job_ref_submission_value`](../../../src/job/parts/dag/p003/body.rs) constructs `job-ref-submission-v1`, including executable and input `job-content-ref` records. Each content record carries a reference, size, format, and optional schema. The parser rejects prohibited inline forms and computes the submission reference from the canonical Preserves value. Rust struct layout is not the identity boundary.

The observable boundary here is a parsed submission, not a started worker. The test checks that the parsed submission contains one input. Its locally generated policy-looking references are fixture inputs, not proof of a current authority decision.

## 3. Inspect preflight without overclaiming admission

`execute_blob_ref_job` parses the submission and gathers preflight diagnostics. The [preflight helper](../../../src/job/parts/dag/p013/body.rs) checks that policy, provenance, and effect-reference collections are nonempty, that the output mode is supported, and that the handler is `local-echo-v1`.

This is an important source-review limit: the helper does not dereference those collections and independently establish current policy, executable provenance, or effect-profile admission. The [governing reference-execution document](../../unison-reference-execution.md) describes a receiver-authoritative admission contract. Do not use this small local fixture as evidence that every part of that broader contract was exercised.

The worker records `queued` and `fetching` statuses before deciding whether the handler may run. A fetching status therefore does not imply that all preflight checks passed.

## 4. Verify and pin the referenced content

For the executable and then each input, the [fetch helper](../../../src/job/parts/dag/p014/body.rs) reads the manifest, compares its total length with the supplied size, verifies the manifest, reads the object, and pins the manifest. It collects verification, fetch, and pin receipt references.

Only input bytes are retained for the echo handler; executable bytes are verified but are not passed to a process launcher. A missing object or size mismatch contributes diagnostics and prevents the preliminary pass needed by the handler. The neighboring `blob_ref_job_submission_denies_missing_ref_before_run` test captures the missing-input boundary: denial is represented by a receipt and no output manifest, rather than a successful empty output.

## 5. Produce and inspect the output

The handler concatenates the input byte vectors in order. With this fixture's single input, the intended output is the five bytes `hello`. This statement comes from both the handler implementation and the checked-in assertion, not an observed run in this documentation batch.

The output path stores bytes under `job-ref-result`, verifies the new manifest, pins it, and records `running` followed by `result-ready`. The fixture reads the resulting object and compares its bytes with `b"hello"`. Review the object through its manifest; the receipt text is not the output payload.

## 6. Close the evidence chain

Cleanup attempts to unpin executable and input manifests. It does not include the output manifest in that input cleanup list. Successful cleanup receipt references are collected; individual unpin errors are ignored by this helper. A cleanup check consequently must not be interpreted as a proof that every pin was removed.

The final status is `complete` or `failed`, and the final receipt is `job-ref-receipt-v1`. With a ledger supplied, statuses and the receipt are imported. The fixture checks the receipt's output-reference list and the presence of a `job-ref-receipt` artifact in the ledger.

Stop the walkthrough at this local evidence boundary. It proves neither live worker delivery nor arbitrary native execution, durable retry safety, current authority, or production readiness. For the distinct recorded DAG exchange, continue with [preparing a recorded worker exchange](preparing-a-recorded-worker-exchange.md).

## Sources

- [Handbook](../README.md)
- [Content references versus inline values](../../technical/envelopes/content-references-versus-inline-values.md)
- [Receiver-authoritative reference execution](../../unison-reference-execution.md)
- [Blob-ref success and missing-input fixtures](../../../src/job/parts/dag/tests/m000/p000/body.rs)
- [Submission construction and worker sequencing](../../../src/job/parts/dag/p003/body.rs)
- [Preflight, output storage, and cleanup](../../../src/job/parts/dag/p013/body.rs)
- [Manifest checks and echo handler](../../../src/job/parts/dag/p014/body.rs)
