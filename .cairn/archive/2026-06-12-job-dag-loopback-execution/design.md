## Context

Existing job DAG slices provide local execution, planning/profile/fusion preview, loopback artifact sync, target admission, and capability-backed admission. The remaining gap before real remote execution is an execution gate: a target executor should run only when presented with a passing admission receipt that still matches the target registry state.

This slice is loopback/local only. It must not introduce Iroh transport, remote worker processes, distributed scheduling, or source-registry reads during execution.

## Goals

- Define canonical `job-execution-request-v1` and `job-execution-receipt-v1` records.
- Require a passing `job-admission-receipt-v1` before loopback execution.
- Verify the admission receipt binds the same job ref, target peer, sync ref, closure refs, authority receipt refs, selected stages, resource refs, and decision.
- Recompute/verify the target artifact closure immediately before execution.
- Execute using the target registry/storage/cache/chunk roots only.
- Emit execution receipts binding admission evidence, sync evidence, authority evidence, stage receipts, outputs, and target peer.

## Non-Goals

- No real network execution.
- No source registry reads during target execution.
- No peer worker process management.
- No arbitrary mobile closure execution.
- No authority from artifact possession alone.
- No bypass around stage-local executor/resource checks.

## Records

```preserves
<job-execution-request-v1 "molten.job-dag.execution-request.v1"
  <job <job-ref>>
  <admission <job-admission-receipt-ref>>
  <target-peer "peer:loopback">
  <stages ["stage-id" ...]>
  <storage <storage-root-ref-or-profile-ref>>
  <cache <cache-root-ref-or-profile-ref>>
  <chunks <chunk-root-ref-or-profile-ref>>
  <policy [<policy-ref> ...]>
  <capability [<authority-context-ref> ...]>
  <resource [<resource-ref> ...]>
  <checks [<check "admission-required" "pass"> ...]>>
```

The request references admission evidence by full canonical ref. Filesystem roots used by the loopback CLI are operational arguments, not semantic identity. The canonical request binds refs/profiles for storage/cache/chunks rather than paths.

```preserves
<job-execution-receipt-v1 "molten.job-dag.execution-receipt.v1"
  <operation "execute-loopback">
  <decision "pass"|"deny">
  <job <job-ref>>
  <request <execution-request-ref>>
  <admission <admission-receipt-ref>>
  <sync <sync-ref>>
  <target-peer "peer:loopback">
  <closure [<artifact-ref> ...]>
  <authority [<authority-admission-receipt-ref> ...]>
  <stages [<stage "id" <stage-receipt-ref>> ...]>
  <outputs [<output-ref> ...]>
  <diagnostics ["..." ...]>
  <refs [<ref> ...]>
  <checks [<check "admission-pass" "pass"> ...]>>
```

Denied receipts are first-class evidence and must explain the failed precondition without executing stage logic.

## Execution Algorithm

1. Parse the execution request.
2. Load the admission receipt from the target ledger/registry or explicit receipt file/ref input.
3. Verify the admission receipt:
   - schema is `job-admission-receipt-v1`;
   - decision is `pass`;
   - job ref and target peer match the execution request;
   - admission refs include sync evidence and authority admission refs;
   - closure refs are non-empty and still present in the target registry;
   - checks include target closure, Trellis topology, executable artifact, capability authority, resource profile, and no-execution from admission.
4. Recompute the target job/stage closure from the target registry and compare it with admitted closure refs.
5. Run the existing local job executor using target registry/storage/cache/chunks only.
6. Emit a canonical execution receipt binding the admission receipt, sync ref, authority receipt refs, stage receipt refs, output refs, and target peer.

## Denial Cases

Execution MUST deny before running if:

- admission receipt is absent or unreadable;
- admission decision is `deny` or any unknown value;
- admission job ref does not match the request;
- target peer does not match;
- admitted closure is missing from the target registry or diverges from recomputed closure;
- authority admission refs are missing;
- required admission checks are absent or fail;
- target storage/cache/chunk operational roots are unavailable;
- selected stages are unknown or unsatisfied.

## CLI

Add:

```text
molten test job execute-loopback \
  <job-ref> \
  --target-registry <path> \
  --storage <path> \
  --cache <path> \
  --chunks <path> \
  --admission-receipt <path-or-ref> \
  --target-peer peer:loopback \
  [--stage <stage-id>] \
  [--out <path>] \
  [--receipt-out <path>]
```

The command must not accept a source registry. This guards against accidentally reading from the source after sync/admission.

## Tests

- Pass after sync plus capability-backed admission.
- Deny without admission receipt.
- Deny with admission receipt decision `deny`.
- Deny when admission receipt job or target peer mismatches the execution request.
- Deny when a closure artifact is removed or changed after admission.
- Assert source registry is not referenced by loopback execution.
- Assert outputs match equivalent local execution for supported deterministic stages.
- Assert execution receipt binds admission, sync, authority, stage receipts, output refs, target peer, and resource refs.
