# Choosing fixture versus live commands

Mode: How-to

## Goal and prerequisites

Choose a command whose actual effects and evidence match your question. The goal is not to find the most realistic-sounding verb. A fixture may exercise real adapters, a planning command may write files without executing operations, and a command nested under `test` may share an operational handler.

Prerequisites are the exact checkout, a source trace to the handler, knowledge of the input's owner, and a fresh isolated output location for any writes. For live work, also obtain the subsystem's actual current-admission and capability requirements; a fixture receipt cannot provide them. This article is source-checked, not runtime-verified. Start at the [Handbook](../README.md); use the [preview-first companion](../../technical/world-effects/preview-first-operator-composition.md) for the distinction between reviewed intent and current facts.

## 1. State the observation you need

Write one narrowly scoped question before selecting a route:

| Question | Appropriate starting point | Insufficient conclusion |
| --- | --- | --- |
| Does this workflow request produce an admitted plan? | World request decoding and planning | Components ran successfully |
| Do time adapters satisfy the fixture's bounded scenarios? | Fabric-time executable fixture | Arbitrary services meet deadlines |
| Can a stored time report be decoded? | Fabric-time report readback | Every linked event was independently replayed |
| What does a live node operation require? | Node declaration, handler, and node authority contract | A parser test or fixture grant authorizes it |

The question determines the evidence owner. An operation called `verify` is not a universal validator for every artifact bearing a `.preserves` suffix.

## 2. Classify by handler, not namespace

Inspect the [root declarations](../../../src/main/root/parts/command/p000/body.rs) and [dispatch](../../../src/main/root.rs). Both `node` and `test node` reach the same node command type and dispatcher. Consequently, `test` is not a general sandbox guarantee. Conversely, the top-level `fabric-time` family contains a fixture runner and report reader rather than a general-purpose daemon controller.

Follow aliases through [main.rs](../../../src/main.rs). At the leaf, identify file reads, file writes, clock or entropy access, transport calls, and state mutation. Separate the pure decision function from the shell supplying observations. The [node-state contract](../../node-state-filesystem-authority.md) explains why an operator-selected state path becomes explicit filesystem authority instead of an incidental output directory.

## 3. Check whether selection restricts execution or reporting

A worked source-review case is `FabricTimeFixtureSelection::DeterministicSimulation`. The name might suggest that live adapters are not touched. The [fixture implementation](../../../src/fabric_time/parts/fixture/p000/body.rs) instead constructs both profiles, runs live and virtual conformance, observes the live clock, and runs production entropy before constructing the selected report. Selection affects report construction and final-time choice; it is not an offline execution switch.

The [existing reproducibility test](../../../src/fabric_time/parts/tests/p000/body.rs) explicitly concerns deterministic report identity despite live adapter execution. Therefore choose this fixture only if those host effects are acceptable. If your requirement is “no live clock or production entropy access,” stop: this CLI route does not establish that requirement. Do not substitute a seed, suppress the host failure, or relabel the result as pure simulation.

## 4. Use a bounded readback when it answers the question

If you already possess a fabric-time run report, readback avoids rerunning the fixture. The recipe below is source-checked and **not executed**. `FABRIC_TIME_REPORT` must name an existing report you are authorized to read, not an event file. Provenance: [Show's positional report declaration](../../../src/cli/runtime/fabric_time/command.rs), [routing](../../../src/cli/runtime/fabric_time.rs), and [readback implementation](../../../src/cli/runtime/fabric_time/ops.rs).

```sh
: "${FABRIC_TIME_REPORT:?Set the path of an existing fabric-time run report}"
molten fabric-time show "$FABRIC_TIME_REPORT"
```

The reader checks file size against 1,048,576 bytes, reads text, parses Preserves, and invokes the run-report parser. Its output is a bounded summary. It does not turn a report into current timing authority or a guarantee of remote deadlines.

## 5. Distinguish preview from executable composition

For world operations, [WorldCommand](../../../src/cli/runtime/world.rs) routes the graph-level plan and single-operation read variants to planning. Mutation variants also plan first. The standalone apply route lacks a live handler registry and fails closed with a denial receipt.

Choose preview when evaluating request shape and planning facts. Choose a live subsystem route only after inspecting that route's concrete adapters and governing operational procedure. There is no safe generic transformation from a retained fixture request to a live workflow: expected head, generation, policy, observations, and profiles must come from real reviewed evidence, not copied fixture hashes.

## 6. Record the decision and stop conditions

Record the selected command's handler, input provenance, output owner, expected effect class, and non-claims. Stop on missing adapters, unsupported profiles, uncertain external outcomes, or unavailable authority. Preserve partial outputs and use component reconciliation for uncertain effects rather than unconditional retries.

A passing fixture is scoped evidence. It does not prove production readiness, global time, distributed lease exclusivity, consensus, or exactly-once effects. This classification is complete when another reviewer can tell what was observed and what was deliberately not attempted.

## Sources

- [Handbook](../README.md)
- [Preview-first operator composition](../../technical/world-effects/preview-first-operator-composition.md)
- [Fabric-time runtime contract](../../fabric-time-scheduler-runtime.md)
- [Node-state filesystem authority](../../node-state-filesystem-authority.md)
- [Root routing](../../../src/main/root.rs)
- [Fabric-time fixture implementation](../../../src/fabric_time/parts/fixture/p000/body.rs)
- [Fabric-time report handler](../../../src/cli/runtime/fabric_time/ops.rs)
