# World command and evidence reference

Mode: Reference

Use this page to route a request or artifact to its owner without confusing orchestration with component execution. It is a source-checked inventory, not a record of commands executed for this batch. The [Handbook](../README.md) supplies procedures; [preview-first composition](../../technical/world-effects/preview-first-operator-composition.md) explains why these surfaces remain separate.

## Standalone workflow surface

The [world declaration](../../../src/cli/runtime/world.rs), exposed through [main aliases](../../../src/main.rs) and [top-level registration](../../../src/main/root/parts/command/p000/body.rs), determines the following spellings. Entries are command names and argument references, not executable recipes.

| Surface | Input and output flags | Standalone meaning |
| --- | --- | --- |
| `world plan` | Required `--request`, `--out`; optional `--receipt-out`, `--summary-out` | Plan a multi-operation graph |
| `world inspect`, `diff`, `conflicts`, `replay`, `simulate`, `verify`, `export`, `gc-plan` | Required `--request`, `--plan-out`; optional receipt and summary outputs | Plan exactly one matching operation; do not infer execution from the verb |
| `world checkpoint`, `branch`, `run`, `promote`, `import` | Required request and plan output; optional summary, receipt, `--apply-plan-ref` | Preview by default; apply requires explicit receipt output and fails closed without handlers |

The workflow CLI does not compose ambient live handlers even for read-shaped operations. The [service](../../../src/world_operator/service.rs) separately exposes planning, handler preview, handler apply, and record publication APIs. A caller using those APIs must supply reviewed ports; CLI availability is not evidence that such composition exists on a host.

## Operation ownership

| Operation kinds | Component owner | Evidence must not be promoted into |
| --- | --- | --- |
| inspect, checkpoint | World Commit | Runtime activation permission |
| branch | World Head | Effect-release permission |
| run, simulate | Fabric Simulation | External execution proof |
| diff, conflicts | World Merge | Merge correctness or branch authority |
| replay, verify, export, import | World Replay | Live fallback permission |
| promote | World Promotion | External effect completion |
| gc-plan | World Distribution | Deletion authority |

A handler registry rejects duplicate kinds and crossed component owners. A matching method signature is insufficient ownership evidence. The [governing workflow contract](../../world-operator-workflows.md) defines the closed mapping and non-claims.

## Request field dictionary

The JSON shape comes from [document.rs](../../../src/cli/runtime/world/document.rs), whose nested structures also reject unknown fields.

| Field group | Real fields | Review use |
| --- | --- | --- |
| Envelope | `schema`, `request_ref`, `world_ref` | Identify the protocol and requested world |
| Mutable fence | `branch_id`, `expected_head`, `expected_generation` | Bind the preview to an expected branch state |
| Admission identities | `policy_ref`, `authority_observation_ref` | Name supplied facts, not durable permission |
| Limits | `limits_ref`, `max_operations`, `max_dependencies_per_operation`, `max_receipt_links`, `max_canonical_bytes` | Bound graph and evidence production |
| Profiles | `profile_ref`, `kind`, `status`, `status_ref` | Bind exact supported/admitted profile evidence |
| Observations | `kind`, `observation_ref`, `subject_ref`, `admitted` | Associate a supplied observation with its subject |
| Operations | `operation_id`, `kind`, `subject_ref`, `profile_ref`, `dependencies` | Identify work and dependency edges |

The fixture schema string is `molten.world-workflow-request.v1`. Canonical record labels are a different vocabulary: `molten-world-workflow-request-v1`, `molten-world-workflow-plan-v1`, `molten-world-workflow-receipt-v1`, and `molten-world-workflow-summary-v1`. Do not replace the JSON schema with a canonical record label.

## Artifact and owner lookup

| Artifact | Producer or owner | What it records |
| --- | --- | --- |
| Workflow plan | Operator planner | Normalized operation ordering and checked planning facts |
| Workflow receipt | Operator aggregation | Component links, completion states, first blocker |
| Workflow summary | Operator renderer | Bounded references, counts, states, blocker codes |
| World commit | World Commit codec/capture | Immutable profile-relative typed-root identity |
| Closure report | World Commit validation | Bounded presence/identity observations |
| Restore plan | Commit or Snapshot owner, depending on record | Ordered proposed restore work; not activation |
| Branch claim | World Head | Expected/successor commits and generations under policy |
| Merge plan/conflict | World Merge | Proposed outputs or unresolved disagreement |
| Promotion plan/reservation | World Promotion | Candidate transition and local release eligibility |
| Attempt/observation | Promotion dispatch bookkeeping | A particular dispatch effort and learned outcome |

Canonical Preserves plus the relevant BLAKE3 identity construction define artifact identity. Rust layout, terminal formatting, or a filename does not. Distinguish a workflow plan reference from a component plan reference even when both appear in the same review packet.

## Component command routing

[World Commit](../../../src/cli/runtime/worldcommit.rs) provides `inspect`, `validate`, `explain`, and `plan-restore` over an explicit positional commit and `--state-root`. [World Snapshot](../../../src/cli/runtime/worldsnapshot.rs) consumes descriptor files; compatibility compares the source against the destination descriptor's cohort. Its standalone restore denies without a runtime adapter.

[World Promotion](../../../src/cli/runtime/worldpromotion.rs) has a separate JSON request format. Its `plan` does not accept the workflow graph. `outbox-inspect` reads reservations; `reconcile` reports unresolved counts without automatic retry. [World Merge](../../../src/cli/runtime/parts/worldmerge/p000/body.rs), included by [worldmerge.rs](../../../src/cli/runtime/worldmerge.rs), compares explicit base/left/right commits and keeps standalone publication disabled.

## Worked lookup: a misleading successful preview

A reviewer receives the logical fixture's `plan.preserves` and a summary mentioning promote. Route that artifact to the operator planner first. The [fixture test](../../../src/cli/runtime/world/tests.rs) creates its expected artifacts through `plan_world_operator_request`, not through a promotion store or effect adapter. Therefore the next requested evidence is the actual component promotion and persistence evidence, not an external completion claim. If no live embedding exists, the review stops at preview availability. A receipt's existence cannot fill that gap.

## Sources

- [Handbook](../README.md)
- [World workflow contract](../../world-operator-workflows.md)
- [Preview-first composition companion](../../technical/world-effects/preview-first-operator-composition.md)
- [Workflow CLI declarations](../../../src/cli/runtime/world.rs)
- [Workflow document definitions](../../../src/cli/runtime/world/document.rs)
- [Workflow service](../../../src/world_operator/service.rs)
- [Logical request fixture](../../../tests/fixtures/world-operator/logical/request.json)
