## Context

Local job DAG execution, planning, profiling, fusion previews, and loopback artifact sync exist. The next distributed boundary is not execution; it is target-side admission. A target must be able to say "this synced job/stage closure is present, canonical, policy-authorized, capability-authorized, and resource-admissible" before any later executor can run a stage.

This slice is local/loopback only. It does not add Iroh transport, remote process launch, worker pools, or distributed scheduling.

## Goals

- Define canonical `job-admission-request-v1`, `job-admission-plan-v1`, and `job-admission-receipt-v1` records.
- Verify job artifact, selected stage artifacts, and dependency closure in the target registry.
- Use Trellis topology/job-DAG primitives for stage ordering and dependency-satisfaction checks.
- Require explicit policy, capability, evidence, and resource refs for admission.
- Deny non-artifact executable configurations, raw/mobile closures, paths, shell commands, inline code, and stale/tampered artifacts.
- Bind resource/profile compatibility without executing stages.
- Emit pass/deny receipts with `no-execution` evidence.

## Non-Goals

- No stage execution.
- No real network transport.
- No remote worker process lifecycle.
- No arbitrary closure shipping.
- No admission by mutable names, paths, mtimes, or display metadata.
- No implicit authority from possession of synced artifacts.

## Records

```preserves
<job-admission-request-v1 "molten.job-dag.admission-request.v1"
  <job <job-ref>>
  <sync <sync-plan-or-receipt-ref>>
  <stages ["stage-id" ...]>
  <target-peer "peer:loopback">
  <policy [<policy-ref> ...]>
  <capability [<capability-ref> ...]>
  <evidence [<evidence-ref> ...]>
  <resource [<resource-ref> ...]>
  <checks [<check "no-execution" "pass"> ...]>>
```

The request binds the synced closure evidence and the explicit authority/resource evidence the target must use. Empty stage selection means all executable stages in the DAG.

```preserves
<job-admission-plan-v1 "molten.job-dag.admission-plan.v1"
  <request <request-ref>>
  <job <job-ref>>
  <sync <sync-plan-or-receipt-ref>>
  <target-peer "...">
  <stages ["stage-id" ...]>
  <closure [<artifact-ref> ...]>
  <topology ["stage-id" ...]>
  <stage-verdicts [<stage "id" "pass-or-deny" [<reason ...>]> ...]>
  <resource-verdict "pass-or-deny">
  <decision "pass-or-deny">
  <checks [<check "target-closure-present" "pass"> ...]>>
```

Plans are advisory. A denied plan is still useful evidence and should explain missing refs, tampered refs, unsupported executable configs, stale policy/capability evidence, and resource failures.

```preserves
<job-admission-receipt-v1 "molten.job-dag.admission-receipt.v1"
  <operation "admit-plan"|"admit-loopback">
  <decision "pass"|"deny">
  <job <job-ref>>
  <request <request-ref>>
  <artifact <admission-plan-ref>>
  <refs [<job-ref> <sync-ref> <closure-ref> ...]>
  <checks [<check "no-execution" "pass"> ...]>>
```

Receipts are canonical pass/deny evidence. They do not grant execution authority by themselves; later execution must reference an admission receipt and still perform executor-local checks.

## Admission Checks

### Target closure

- Resolve the job artifact in the target registry by full artifact ref.
- Parse the job DAG payload and verify its canonical job ref.
- Recompute selected stages and dependency closure roots from the target job DAG.
- Verify all closure artifacts are present in the target registry and match their refs.
- Verify the closure agrees with the referenced sync plan/receipt when available.

### Topology

- Derive canonical stage order with Trellis topology primitives.
- Reject cycles, unsatisfied selected-stage dependencies, unknown stage ids, and topology divergence.
- Stage selection may restrict execution, but all selected dependencies must be satisfied by selected stages or already-materialized inputs.

### Executable boundary

- Stages that require execution must name an artifact-backed executable/stage operation admitted at the target.
- Reject raw closures, inline scripts, host paths, shell commands, mutable image tags, unverified remote URLs, or config records that would require shipping code outside artifact refs.
- Built-in local planning/profile/materialization records remain advisory and do not satisfy target executable admission unless explicitly represented as admitted artifacts or target-native built-ins with policy evidence.

### Authority and policy

- Require non-empty explicit policy refs, capability refs, evidence refs, and resource refs for pass admission.
- Verify refs are canonical and available in the target ledger/registry as applicable.
- Bind the target peer identity and do not allow authority to transfer solely from source possession or sync success.
- Keep future Nickel/Basalt/UCAN checks behind explicit evidence refs; missing or stale evidence denies.

### Resources

- Use the existing job profile/resource governance records to estimate stage resource needs.
- Check selected stage requirements against target resource refs.
- Deny over-budget stages before execution.
- Receipt checks must bind resource profile refs and resource admission refs.

## CLI

- `molten test job admit-plan`
  - Reads target registry and a job/admission request shape.
  - Writes `job-admission-plan-v1` and a plan receipt.
- `molten test job admit-loopback`
  - Performs local target admission verification after loopback sync.
  - Writes pass/deny `job-admission-receipt-v1`.

## Tests

- Admission passes after sync for an artifact-backed executable stage closure with explicit policy/capability/evidence/resource refs.
- Admission denies when the target is missing a dependency closure member.
- Admission denies when a target artifact envelope is tampered or does not hash to the requested ref.
- Admission denies raw/mobile closure stage configs and path/shell/url executable configs.
- Admission denies when policy, capability, evidence, or resource refs are absent.
- Admission receipts include `no-execution`, target closure, topology, authority, and resource checks.
