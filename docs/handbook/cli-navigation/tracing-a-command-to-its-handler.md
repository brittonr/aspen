# Tracing a command to its handler

Mode: Walkthrough

This walkthrough follows the checked-in logical world-operator request from command parsing to published planning records. It is a source-checked navigation exercise, not an execution report. No command or test below was executed for this article. Use the [Handbook](../README.md) for practical routes and the [preview-first companion](../../technical/world-effects/preview-first-operator-composition.md) for the theory behind planning and fresh apply admission.

## 1. Choose the input before choosing the verb

Open the [logical request](../../../tests/fixtures/world-operator/logical/request.json). It names branch `world/operator-dogfood`, expected generation 11, explicit observations, resource limits, and a dependency chain of thirteen operations. The chain includes inspect, checkpoint, branch, simulate, run, diff, conflicts, replay, verify, promote, export, import, and garbage-collection planning.

Those references belong to a retained fixture. They are not credentials or current observations of an operator's deployment. The exercise consumes the file unchanged; it does not turn its admitted observation flags into live authority.

Because the request contains multiple operations, select the graph-level `world plan` route. A single-operation command such as `world inspect` is the wrong consumer even though inspect is the first operation. The boundary to observe is request cardinality, not whether the command name appears somewhere in the graph.

## 2. Resolve the executable and root parser

[The entrypoint](../../../src/main.rs) calls `cli_root::run`. Its path alias maps `entrypoint` to `main/root.rs`; `cli_root` re-exports that module. Do not look for a guessed `cli_root.rs` file.

[The root dispatcher](../../../src/main/root.rs) parses `command::Cli`. Its [command module](../../../src/main/root/command.rs) includes two parts: [p000](../../../src/main/root/parts/command/p000/body.rs) declares the parser and enums; [p001](../../../src/main/root/parts/command/p001/body.rs) contains parser tests. The parser name is `molten`, and `Top::World` delegates its nested command type to `crate::cli_world_operator::WorldCommand`.

The observable boundary here is Clap acceptance. A parser test accepting a request pathname says nothing about that file's existence or admission.

## 3. Follow the alias into the actual handler

Back in `main.rs`, `cli_world_operator` re-exports `world_operator_port`, whose path is `cli/runtime/world.rs`. The root match calls `run_world_command`; its `Plan` arm reaches `plan_complete`.

The [world declaration](../../../src/cli/runtime/world.rs) requires `--request` and `--out` for this variant. `--receipt-out` and `--summary-out` are optional. The spelling differs from single-operation variants, which require `--plan-out`. Follow the specific argument struct rather than transferring flags between siblings.

The following source-checked recipe is **not executed**. It assumes `molten` is already available and `CLI_TRACE_OUT` names an existing, fresh, isolated directory whose three destination files do not exist. The guard only checks that the variable is set; review the directory before use. Provenance: [WorldPlanArgs and dispatcher](../../../src/cli/runtime/world.rs), [root parser](../../../src/main/root/parts/command/p000/body.rs), and [output writer](../../../src/cli/runtime/world/output.rs).

```sh
: "${CLI_TRACE_OUT:?Set an existing fresh isolated output directory}"
molten world plan \
  --request tests/fixtures/world-operator/logical/request.json \
  --out "$CLI_TRACE_OUT/plan.preserves" \
  --receipt-out "$CLI_TRACE_OUT/receipt.preserves" \
  --summary-out "$CLI_TRACE_OUT/summary.preserves"
```

## 4. Separate decoding from planning

[The document adapter](../../../src/cli/runtime/world/document.rs) reads bytes, decodes JSON with unknown-field rejection, and constructs typed references and closed operation/profile values. This is distinct from [the planner service](../../../src/world_operator/service.rs), which calls core planning and constructs a run with no component receipt links.

A valid JSON object can therefore fail typed conversion; a typed request can still fail planning. Preserve the original input when diagnosing either boundary. Do not “repair” a reference by substituting a syntactically convenient hash.

## 5. Account for each output

[The writer](../../../src/cli/runtime/world/output.rs) writes canonical plan bytes, then optional receipt bytes, then optional summary bytes, followed by terminal presentation. It does not create missing parent directories or make these writes one transaction. An error after the first write may leave a partial artifact set.

The artifacts describe the planning result, not executed checkpoint, promotion, export, or deletion effects. Canonical Preserves plus BLAKE3 define identity; filenames and Rust struct layout do not. The [retained-fixture test](../../../src/cli/runtime/world/tests.rs) compares generated plan, receipt, and summary bytes with checked-in records. That is existing test intent, not a newly observed passing result.

## 6. Stop at the composition boundary

Changing to a mutation verb and supplying an apply reference does not install live component handlers. The standalone CLI writes a denial receipt and returns an error; matching and stale references select different blockers. The [governing workflow contract](../../world-operator-workflows.md) explicitly requires reviewed handlers and fresh-facts adapters for an embedding.

The completed trace is therefore input → parser → typed adapter → planner → planning artifacts. Live workflow execution is intentionally outside this walkthrough. A useful review note records that boundary rather than treating a successful preview as a deployment tutorial.

## Sources

- [Handbook](../README.md)
- [Preview-first operator composition](../../technical/world-effects/preview-first-operator-composition.md)
- [World operator workflow contract](../../world-operator-workflows.md)
- [CLI aliases and process exit](../../../src/main.rs)
- [World command and argument declarations](../../../src/cli/runtime/world.rs)
- [Logical request fixture](../../../tests/fixtures/world-operator/logical/request.json)
- [Fixture and denial tests](../../../src/cli/runtime/world/tests.rs)
