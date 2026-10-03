# Following a world preview

Mode: Walkthrough

This walkthrough follows the checked-in logical workflow request through the standalone planner. Its finish line is a reviewable plan, receipt, and summary, not a running world. Use the [Handbook](../README.md) for other practical paths and [preview-first composition](../../technical/world-effects/preview-first-operator-composition.md) for the underlying model.

**Execution status:** source-checked, not executed for this documentation batch. The retained artifacts are fixtures; their presence is not fresh execution evidence. The command below requires an available `molten` binary and an existing, isolated output directory.

## 1. Establish which input is being followed

Open [logical/request.json](../../../tests/fixtures/world-operator/logical/request.json). It identifies branch `world/operator-dogfood`, expected generation 11, an expected head, policy and authority-observation references, one admitted logical profile, and explicit observations. These are fixture facts, not observations of your machine or grants to mutate another branch.

The request bounds are 32 operations, 32 dependencies per operation, 64 receipt links, and 262144 canonical bytes. Its 13 operations form a dependency chain: inspect, checkpoint, branch, simulate, run, diff, conflicts, replay, verify, promote, export, import, and gc-plan. Follow each dependency by operation identity rather than by array position. The fixture's `run` operation belongs to Fabric Simulation; do not relabel it as a second checkpoint merely because the high-level dogfood description mentions capturing successor work.

**Observable boundary:** you have a JSON planning request. No root objects, live adapters, or current authority have been established by reading it.

## 2. Follow parsing before planning

The [CLI document adapter](../../../src/cli/runtime/world/document.rs) uses closed deserialization structures with unknown-field rejection. It converts strings into typed references and closed operation/profile vocabularies. The request is not a place for raw shell commands or arbitrary component options.

The full graph belongs to `world plan`. Passing this same document to `world checkpoint` would fail the typed-command requirement: non-plan commands require exactly one operation of the matching kind. A useful checked-in boundary example is the [CLI test](../../../src/cli/runtime/world/tests.rs), which selects the checkpoint operation and clears its dependencies before testing single-operation apply denial. Simply dropping predecessor operations without removing their dependency references does not produce an equivalent valid request.

**Observable boundary:** parsing can reject the document before any canonical plan exists. Preserve that distinction when recording a failure.

## 3. Produce fresh preview artifacts

Choose an existing isolated directory for `WORLD_PREVIEW_DIR`; the CLI uses ordinary file writes, so reused filenames can replace prior evidence. Run from the repository root. This source-checked recipe has not been executed here. Spelling and outputs come from the [world command declaration](../../../src/cli/runtime/world.rs), [top-level registration](../../../src/main/root/parts/command/p000/body.rs), [main aliases](../../../src/main.rs), and [output implementation](../../../src/cli/runtime/world/output.rs).

```sh
molten world plan \
  --request tests/fixtures/world-operator/logical/request.json \
  --out "${WORLD_PREVIEW_DIR:?Set an existing isolated output directory}/plan.preserves" \
  --receipt-out "${WORLD_PREVIEW_DIR}/receipt.preserves" \
  --summary-out "${WORLD_PREVIEW_DIR}/summary.preserves"
```

The plan records normalized planning facts and ordering. The receipt describes the aggregate planning result; it is not evidence of executed checkpoints or promotions. The summary is a bounded diagnostic view. The [planning service](../../../src/world_operator/service.rs) builds this run without component links by calling `build_run` with an empty link collection. Although the service also constructs a canonical request record, this CLI invocation does not offer a request-record output flag.

**Observable boundary:** filesystem publication of these three artifacts is separate from world mutation. Output files alone cannot establish component completion.

## 4. Compare the right things

The [retained-fixture test](../../../src/cli/runtime/world/tests.rs) recomputes the plan, receipt, and summary from the request and compares canonical bytes with the checked-in artifacts. That test documents the intended comparison boundary; it has not been run for this batch. Use the request and its associated artifacts together when investigating a difference, rather than comparing only terminal text or a count.

Identity belongs to canonical Preserves and domain-separated BLAKE3 constructions, not JSON whitespace or Rust layout. A changed input fact can require a new reviewed preview even if the resulting operation names look unchanged.

## 5. Stop at the apply boundary

The standalone command has no live handler registry. A mutation apply submission with the exact plan reference still reaches a handler-unavailable denial; a different reference produces stale-plan denial. An explicit receipt output is required. No apply command is supplied here because the fixture does not establish a live composition.

The embedding API is a different boundary: it validates handler ownership, previews components, obtains fresh facts before mutations, and stops on blocked or unknown outcomes. It does not roll back earlier component effects or automatically retry uncertain promotion. The practical completion statement for this walkthrough is therefore “preview evidence inspected,” never “world captured and promoted.”

## Sources

- [Handbook](../README.md)
- [World operator workflow contract](../../world-operator-workflows.md)
- [Preview-first composition companion](../../technical/world-effects/preview-first-operator-composition.md)
- [Logical request fixture](../../../tests/fixtures/world-operator/logical/request.json)
- [CLI fixture and denial tests](../../../src/cli/runtime/world/tests.rs)
- [World operator service](../../../src/world_operator/service.rs)
