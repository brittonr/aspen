# Inspecting control-plane coordination

Mode: How-to

## Goal and prerequisites

Use this procedure to inspect coordination evidence without mistaking a local control-registry model for a deployed consensus service. Prerequisites are a source-matched `molten` executable, access to the artifact being reviewed, and a decision about whether you need read-only inspection or a fresh fixture output. Commands below are source-checked but were not executed for this documentation batch. No working development shell is implied.

The [Handbook](../README.md) supplies navigation. The [README coordination UX](../../../README.md#coordination-control-plane-ux) introduces manifest/request/apply composition; this guide instead concentrates on deciding what the resulting evidence can support. For the theory of consistency claims, see the [technical companion](../../technical/membership/consistency-and-fastpath-nonclaims.md).

## 1. Choose inspection before generation

If someone supplied a report, inspect that report first. Set `COORDINATION_ARTIFACT` to its existing path; do not use a receipt reference as a filesystem path. This command only reads and summarizes the selected artifact.

Spelling is backed by the [root declaration](../../../src/main/root/parts/command/p000/body.rs), [coordination declaration](../../../src/cli/workflow/coordination/command.rs), and [show handler](../../../src/cli/workflow/coordination/ops.rs). Source-checked, not executed:

```sh
molten test coordination show "${COORDINATION_ARTIFACT:?Set an existing coordination artifact path}"
```

The summary is a navigation aid, not complete verification. Open the Preserves record and resolve its referenced evidence from the supplied bundle. Keep the original report, source revision, and origin information together. A parsed `pass` does not establish fresh authority or an independently verified live commit.

## 2. Decide whether the fixture answers your question

For learning the local evidence shape, use `run-fixture`. Set `COORDINATION_OUT` to a new, unused directory reserved for this inspection. Confirm it is unused before execution: the writer creates directories and writes files, rather than enforcing exclusive creation. Do not point it at an earlier investigation or node state directory.

The [CLI declaration](../../../src/cli/workflow/coordination/command.rs), [fixture handler](../../../src/cli/workflow/coordination/ops.rs), and [file writer](../../../src/cli/workflow/coordination/io.rs) support this command. Source-checked, not executed:

```sh
molten test coordination run-fixture \
  --out "${COORDINATION_OUT:?Set a fresh isolated output directory}"
```

The handler writes `report.preserves` and indexed `evidence-N.preserves` files. The filename index is serialization order, not semantic identity. Use canonical record references to associate requests, receipts, state snapshots, tokens, and assertions.

If the question is whether a real peer joined a real group, stop: this command is the wrong evidence source. `new_coordination_runtime` constructs `new_control_registry_model_runtime` from a fixture manifest. The path does not select a live cluster from the user's `control_group_ref`.

## 3. Read the worked fixture as mixed evidence

The [fixture request list](../../../src/coordination/parts/mod/p011/body.rs) acquires `resource:alpha` for `client-a`, repeats that operation, attempts a release as `client-b` with token zero, enqueues and dequeues `job-1` on `queue:work`, then registers and reads `svc:api`.

Review these cases separately. The duplicate acquire should lead you to duplicate-replay evidence; the release is a negative fencing case, not an instruction to repair state. The fixture report is constructed with decision `pass`, while the fixture intentionally includes denial behavior. Therefore, its top-level decision is not a claim that every contained request passed. Inspect individual receipts and their diagnostics before explaining the run.

This also distinguishes fixture reports from apply reports. The CLI batch `apply` initializes a fresh runtime, processes requests in order, and changes its aggregate decision to `deny` if any result is nonpassing. It is not an all-or-nothing transaction: prior requests may already have advanced the in-memory model when a later request denies.

## 4. Resolve the replay relationship

For an exact duplicate, compare both operation identity and canonical request reference. The [duplicate implementation](../../../src/coordination/parts/mod/p013/body.rs) emits a new receipt with transition kind `duplicate-replay`, a `prior_receipt_ref`, and preserved-state evidence. It returns prior output references without a new Raft commit or new status assertions. It does not simply return a byte-identical copy of the original receipt.

A reused operation identifier with a different request reference is instead a conflicting duplicate denial. Preserve both requests and the original receipt; do not change identifiers merely to evade that conflict. Determine the intended operation before preparing a genuinely new request.

## 5. Stop at the observed boundary

Record which artifacts you actually inspected and whether execution occurred. Distinguish declared `linearizable` versus `local-stale` read mode, model commit evidence, status assertions, and live adapter evidence. A dataspace status assertion is an observation, not admission authority. Re-running `apply` starts another fresh model and does not demonstrate durable deduplication across process restarts.

Conclude with the narrow finding: for example, “the supplied duplicate receipt binds the prior receipt and preserves the model state.” Do not conclude exactly-once external work, current membership, or release readiness from it.

## Sources

- [Handbook](../README.md)
- [Coordination UX](../../../README.md#coordination-control-plane-ux)
- [Consistency and fast-path non-claims](../../technical/membership/consistency-and-fastpath-nonclaims.md)
- [Main module aliases](../../../src/main.rs)
- [Root command dispatch](../../../src/main/root.rs)
- [Coordination declarations](../../../src/cli/workflow/coordination/command.rs)
- [CLI apply, fixture, and show handlers](../../../src/cli/workflow/coordination/ops.rs)
- [Model runtime construction](../../../src/coordination/parts/mod/p002/body.rs)
- [Duplicate receipt construction](../../../src/coordination/parts/mod/p013/body.rs)
