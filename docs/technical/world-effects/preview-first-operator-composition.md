# Preview-first operator composition

The `molten world` family composes existing world components without introducing a second runtime or workflow engine. This article assumes familiarity with branch heads, profiles, and component receipts. It explains how preview identity, fresh observations, and bounded aggregation work together under the [operator workflow contract](../../world-operator-workflows.md). The [Technical companion](../README.md) links the component-level discussions.

## Composition retains domain ownership

The operator owns typed requests, deterministic ordering, stable preview identities, first-blocker selection, fresh apply admission, receipt links, and summaries. It does not absorb component semantics. World Commit owns inspect and checkpoint; World Head owns branch creation; Fabric Simulation owns run and simulate; World Merge owns diff and conflicts; World Replay owns replay, verify, export, and import; World Promotion owns promote; World Distribution owns garbage-collection planning.

The [shell service](../../../src/world_operator/service.rs) validates the handler registry before preview or apply. Each handler's owner must match its operation kind, and duplicate operation kinds are rejected. This is more precise than accepting any adapter with a compatible method signature: an inspect handler cannot relabel itself as the head owner and thereby change the meaning of its receipt.

The functional core operates on supplied facts without performing file, process, network, clock, credential, storage, or component operations. The shell loads requests and composes ports. Therefore deterministic operation ordering is an in-memory property, whereas the accuracy of a current-head observation depends on the explicitly supplied shell adapter.

## Preview binds the request, not future reality

CLI requests are explicit JSON, reject unknown fields, and name identities, expected head and generation, policy and authority observations, resource limits, profiles, and operation dependencies. The planner rejects malformed graphs and normalizes operation order independently of input order. A stable preview identity represents the normalized admitted planning facts; it is not a timeless permission token.

`apply_world_operator_with_handlers` recomputes the plan and compares its reference with the submitted preview reference before execution. A mismatch creates a stale-plan blocker. During execution, each available handler is previewed again. Only a planned preview proceeds. Before each operation classified as mutating, the shell obtains fresh facts and calls core apply admission. A mismatch becomes a mutable-observation-drift blocker before that component's execute call.

Two fences are therefore distinct. Exact preview identity prevents applying a different request from the reviewed one. Fresh mutable observations prevent applying an unchanged request against a changed world. Neither fence alone substitutes for the other. This structure is visible directly in the [apply loop](../../../src/world_operator/service.rs).

## Illustrative checkpoint race

Suppose an operator previews a graph containing inspect, checkpoint, branch, replay, promote, export, and retention planning. The preview binds branch generation 20. These operation labels and generation are illustrative rather than a runnable request.

Before checkpoint executes, another admitted operation advances the branch to generation 21. The submitted preview identity can still match the original request, but fresh facts no longer match its expected generation. Apply stops before checkpoint mutation. Earlier inspection evidence can remain valid evidence of what was inspected; it does not authorize checkpoint against the new head. Later branch and promotion operations cannot be reported complete.

Now consider a different run where earlier operations finish but promotion returns an unknown outcome. The workflow stops before export and links the component evidence instead of retrying promotion. A retry could duplicate uncertain external work or misclassify durable publication. Resolution belongs to the [promotion reconciliation boundary](../../world-promotion-and-effect-release.md), not to a generic workflow retry loop.

## Aggregate evidence remains evidence

The workflow emits canonical request, plan, receipt, and summary records. Receipt links retain operation identity, component reference, evidence role, and completion state. The service builds the aggregate through core validation rather than merely concatenating handler output. Links cannot overclaim authority, deletion authority, or sensitive material under the governing contract.

First-blocker behavior constrains the aggregate's meaning. A stopped prefix is not an all-or-nothing rollback of previous components. Earlier complete operations may have real shell effects; later operations are simply not claimed complete. In the [existing shell tests](../../../src/world_operator/tests.rs), an unknown promotion result prevents even the later export preview from running.

Profile states are also explicit: admitted, blocked, unsupported, or unavailable. An unavailable witnessed-head profile is not replaced with local-head behavior. Opaque replay requires its exact profile and rejects semantic diff or conflict comparison. The [snapshot contract](../../world-execution-snapshots.md) explains the underlying incompatibility; workflow composition cannot manufacture equivalence between profiles.

## Verification and limits

Suggested verification, not executed test evidence: inspect stale-plan, stale-generation, missing-handler, unknown-outcome, crossed-owner, sensitive-link, and opaque-semantic-operation cases in the existing tests. Observe the boundary calls that did not happen, not only the final status. When reviewing an embedding, inspect each current-facts adapter and each handler's component delegation separately from the normalized graph.

The standalone CLI has no ambient live handler registry. Its apply path writes a denial receipt and fails closed until reviewed composition exists; producing a preview is not evidence that a host can apply it. Human summaries intentionally expose stable references, counts, states, and blocker codes rather than payloads, credentials, or host paths.

Workflow evidence establishes checked ordering, bindings, and bounded observations. It does not prove component correctness, external completion, runtime safety, release eligibility, or deletion authority. In particular, a final garbage-collection plan remains subject to the independent [retention boundary](../../world-distribution.md), even after every preceding workflow operation reports complete.

## Sources

- [World operator workflows](../../world-operator-workflows.md)
- [World promotion and effect release](../../world-promotion-and-effect-release.md)
- [World execution snapshot profiles](../../world-execution-snapshots.md)
- [World distribution and retention](../../world-distribution.md)
- [Operator shell composition](../../../src/world_operator/service.rs)
- [Operator boundary tests](../../../src/world_operator/tests.rs)
- [Technical companion](../README.md)
