# Command families and artifact reference

Mode: Reference

This is a routing and artifact-ownership desk reference, not a replacement for leaf declarations or a claim that every listed family is production-ready. Spellings and behaviors are source-checked; no commands or tests were executed for this article. The [Handbook](../README.md) provides task-oriented routes. For planning semantics, consult the [technical companion](../../technical/world-effects/preview-first-operator-composition.md).

## Root navigation

The executable parser is named `molten`. [main.rs](../../../src/main.rs) maps modules with `#[path]` and re-exports readable `cli_*` aliases. [root.rs](../../../src/main/root.rs) parses and dispatches; [command.rs](../../../src/main/root/command.rs) includes declaration part `p000` and parser-test part `p001`. A filesystem search for the alias name alone can miss the implementation.

These rows are navigation keys, not runnable recipes. Family presence proves a parser route, not the behavior of every nested verb.

| Root family or group | CLI owner resolved by `main.rs` | Routing purpose |
| --- | --- | --- |
| `cluster`, `dogfood`, `receipts` | `cli/ops/cluster.rs`, `cli/ops/dogfood/mod.rs`, `cli/evidence/receipts.rs` | Cluster, operational rails, and receipt surfaces |
| `node`, `peer` | `cli/ops/node.rs`, `cli/ops/peer.rs` | Node and peer operations |
| `runtime config` | `main/root.rs` | Read an exported startup configuration and summarize it |
| `fabric-time` | `cli/runtime/fabric_time.rs` | Executable time fixture and report readback |
| `fabric-simulation`, `system-extension` | Corresponding modules under `cli/runtime/` | Simulation and extension surfaces |
| `world` | `cli/runtime/world.rs` | Request planning and fail-closed standalone apply |
| `world-commit`, `world-snapshot`, `world-authority`, `world-head` | `worldcommit.rs`, `worldsnapshot.rs`, `worldauthority.rs`, `worldhead.rs` under `cli/runtime/` | Component-owned world surfaces |
| `world-distribution`, `world-merge`, `world-promotion` | `worlddistribution.rs`, `worldmerge.rs`, `worldpromotion.rs` under `cli/runtime/` | Distribution, comparison, and promotion surfaces |
| `test` | `run_test_command` in `main/root.rs` | Nested harness, artifact, workflow, runtime, and evidence routes |

With no subcommand, the root dispatcher prints a greeting. That is not a daemon startup or configuration validation.

## Frequently confused nested routes

The [test enum](../../../src/main/root/parts/command/p000/body.rs) declares positional suite input for `test run` and positional report input for `test replay`. Their optional destinations are `--report-out` and `--failure-out`, respectively. The same enum also nests `report`, `gate`, `receipt`, `ledger`, `chain`, `chunk`, `storage`, `artifact`, `schema`, `cache`, `transcript`, `catalog`, and additional workflow/runtime families. Their specific leaf contracts are independent; an output option on one is not inherited by another.

`node` and `test node` dispatch to the same implementation. `receipts` at the root and `test receipt` use distinct command types from the receipts module. `test raft` remains a control-plane-oriented family; its presence does not imply OpenRaft, general actor-message consensus, or exactly-once delivery.

## Inspected leaf argument shapes

Provenance for the table is [WorldCommand and argument structs](../../../src/cli/runtime/world.rs), [FabricTimeCommand](../../../src/cli/runtime/fabric_time/command.rs), and [root Runtime/Test declarations](../../../src/main/root/parts/command/p000/body.rs).

| Route | Required inputs/destinations | Optional or bounded selections |
| --- | --- | --- |
| `world plan` | `--request`, `--out` | `--receipt-out`, `--summary-out` |
| World single-operation read variants | `--request`, `--plan-out` | `--receipt-out`, `--summary-out` |
| World mutation variants | `--request`, `--plan-out` | `--summary-out`, `--apply-plan-ref`, `--receipt-out`; apply requires receipt output at runtime |
| `fabric-time run-fixture` | No required flag | `--profile`: `live`, `deterministic-simulation`, `both`; `--out` selects a directory |
| `fabric-time show` | Positional report path | Reader enforces a 1,048,576-byte metadata limit |
| `runtime config` | `--config` | Handler reads Nickel-export JSON, not source Nickel evaluation |

World read variants include inspect, diff, conflicts, replay, simulate, verify, export, and gc-plan. Mutation variants are checkpoint, branch, run, promote, and import. These classifications come from the CLI enum and routing, not ordinary-language guesses about the verbs.

## Artifact owners and representations

| Artifact | Producer/consumer | Meaning and important limit |
| --- | --- | --- |
| World JSON request | `world/document.rs` | Explicit facts; unknown fields denied; not a live authority grant |
| World plan destination | `world/output.rs` | Canonical plan bytes; not proof of component execution |
| World receipt destination | Same writer and denial writer | Planning evidence or denial; apply can replace the initially written receipt |
| World summary destination | Same writer | Canonical summary bytes, separate from terminal rendering |
| `profiles/live.preserves` and `profiles/deterministic-simulation.preserves` | Fabric-time artifact planner | Both profile records are included in the output plan |
| `report.preserves` | Fabric-time fixture/readback | Preserves text encoding of the canonical run value |
| `evidence/` numbered event files | Fabric-time artifact planner | Individual event values, not acceptable substitutes for the run report |

The world writer writes canonical record bytes; the time writer renders values using `to_text`. Therefore `.preserves` does not alone tell a generic reader which representation to expect. Canonical identity is defined by Preserves and BLAKE3, never by pathname or Rust DTO layout.

## Worked lookup

Suppose an operator has a time event and wants a summary. The table identifies `fabric-time show` as a run-report consumer, so supplying that event is a type-boundary error, not evidence of corrupt time execution. Locate the enclosing `report.preserves` instead. Likewise, the logical world request contains a graph: route it to `world plan`, not `world inspect`. The [retained request tests](../../../src/cli/runtime/world/tests.rs) establish the intended graph-to-record comparison, without authorizing live application.

## Sources

- [Handbook](../README.md)
- [Preview-first operator composition](../../technical/world-effects/preview-first-operator-composition.md)
- [World operator contract](../../world-operator-workflows.md)
- [Entrypoint aliases](../../../src/main.rs)
- [Root declarations](../../../src/main/root/parts/command/p000/body.rs)
- [World output writer](../../../src/cli/runtime/world/output.rs)
- [Fabric-time artifact planner and reader](../../../src/cli/runtime/fabric_time/ops.rs)
