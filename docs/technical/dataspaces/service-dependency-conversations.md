# Service Dependency Conversations

A service dependency conversation separates wanting a service, identifying its implementation, admitting startup, and observing readiness. The [architecture](../../architecture.md) places demand, readiness, failure, restart, and exposed service objects in the dataspace rather than treating a service name as authority. This article follows the inspected local service-demand and supervision models; it does not describe a production process manager. Return to the [Technical companion](../README.md).

## Demand does not mean permission

The service record layer distinguishes `ServiceDemand`, `ServiceManifest`, and `ServiceStatus`. A demand carries a demand identifier, service identifier, requester reference, optional manifest reference, and policy references. A manifest supplies dependencies and references for ownership, target, provided assertions, restart policy, resource policy, and effect profile. A status links demand references, dependency-status references, readiness assertions, failures, and replay references. These are separate records because each answers a different question. See the [record definitions](../../../src/service/parts/records/p000/body.rs).

The demand serializer includes a `startup-admission-required` check, and its parser requires that check. This records the boundary; it does not run a target process or convert a requester string into capability authority. Canonical hashing identifies the represented record, while semantic authorization remains the responsibility of the relevant admission path. The [record constructors and parsers](../../../src/service/parts/records/p001/body.rs) show that separation.

## Dependency resolution as bounded progress

The demand runtime constructs a map of supplied ready statuses. It includes statuses whose state is `"ready"` and rejects duplicate ready statuses for a service identifier. Dependency resolution then collects ready status references for each manifest dependency. This is evidence linking within the supplied suite, not an ambient health check or proof that a remote dependency remains reachable.

For each demand, the runtime first resolves a manifest. If the demand pins a different manifest reference, it produces a denial rather than silently substituting the resolved manifest. If fewer dependency-status references are available than declared dependencies, the demand remains pending. Only after dependency readiness does startup admission run. The [runtime pass implementation](../../../src/service/parts/runtime/p001/body.rs) makes this order explicit.

The runtime repeats passes while pending demands remain and progress occurs, subject to its bounded pass handling. If no progress occurs, remaining demands produce dependency-wait diagnostics. A detected dependency cycle takes a denial path instead. Cycle detection traverses declared manifest dependencies with a seen set and an explicit graph bound; this finite analysis is not a liveness proof for a changing distributed graph. See [dependency helpers](../../../src/service/parts/runtime/p003/body.rs).

## Startup evidence and readiness publication

`startup_admission_diagnostics` checks for nonempty authority, policy, resource, effect-profile, and source-gate reference collections. These checks are meaningful fail-closed inputs to this local model, but they are presence checks. They must not be restated as complete cryptographic verification, policy evaluation, or live host admission merely because the resulting lifecycle record contains references.

On the admitted path, `start_outcome` constructs a replay identity and readiness assertion, wraps the readiness value in `RuntimeValue`, and applies a `RuntimeStep::Assert` under the manifest's service identifier. It constructs a ready status binding the demand, dependency statuses, readiness reference, and replay identity. This connects a service conversation to the ordinary owned assertion mechanism rather than bypassing it. The [startup outcome source](../../../src/service/parts/runtime/p002/body.rs) shows the assertion and record construction.

## Worked reasoning: frontend waits for backend

Consider an illustrative suite with `svc:frontend` depending on `svc:backend`, demands for both, no initial ready statuses, and all required evidence categories supplied.

If the frontend demand is examined first, it remains pending because the backend has no ready status. The backend has no dependencies and can take the admitted startup path. Its generated ready status enters the map. A later pass can resolve the frontend dependency and publish frontend readiness bound to that backend status reference.

Now remove the backend demand while keeping its manifest. A manifest says what could provide the service; it is not a ready status. The frontend therefore waits rather than manufacturing readiness from artifact availability. If the frontend demand pins an inconsistent manifest, it is denied rather than waiting for a readiness fact to repair that mismatch. If both manifests require each other, cycle handling denies the unresolved conversation rather than iterating indefinitely.

The [demand runtime tests](../../../src/service/parts/runtime/p004/body.rs) exercise two-service startup, unmet dependencies, and cycles. This article describes those sources, not an executed fixture transcript.

## Failure and restart are another conversation

The supervision model separately evaluates restart authority evidence, resource evidence, revocation evidence, attempt budget, and logical backoff. `evaluate_restart` denies revoked or missing authority, missing resources, and exhausted attempts; it returns backoff when the logical slot has not elapsed. Its slot uses checked multiplication of restart attempt and configured backoff steps. These are deterministic logical decisions in [supervision source](../../../src/service/parts/supervision/p003/body.rs), not sleeps or proof of a restarted operating-system process.

A restart decision also cannot prove exactly-once recovery of external work. The supervision report can carry scheduled demands, cleanup receipts, and retractions; actual execution, effect reconciliation, and authority checks remain separate responsibilities. As with the [Syndicate reference boundary](../../syndicate-reference-harness.md), explanatory observations and canonical evidence do not import authority from another runtime's conventions.

## Verification and limits

Suggested review traces one demand through manifest resolution, dependency references, admission diagnostics, readiness assertion, and resulting status. Negative cases should remove each evidence category independently, supply stale or conflicting manifest pins, omit dependency readiness, and exhaust restart attempts. Review what the caller validates about supplied ready statuses rather than assuming status text proves health.

This local model does not establish continuous dependency monitoring, remote freshness, process isolation, durable orchestration, automatic recovery of arbitrary side effects, or production readiness. The architecture's service conversation is broader than the inspected fixture mechanics; this article intentionally keeps those boundaries visible.

## Sources

- [Architecture and service assertions](../../architecture.md)
- [Syndicate reference authority boundary](../../syndicate-reference-harness.md)
- [Service record definitions](../../../src/service/parts/records/p000/body.rs)
- [Demand serialization and parsing](../../../src/service/parts/records/p001/body.rs)
- [Demand resolution passes](../../../src/service/parts/runtime/p001/body.rs)
- [Readiness assertion construction](../../../src/service/parts/runtime/p002/body.rs)
- [Dependency and admission helpers](../../../src/service/parts/runtime/p003/body.rs)
- [Dependency runtime tests](../../../src/service/parts/runtime/p004/body.rs)
- [Logical restart evaluation](../../../src/service/parts/supervision/p003/body.rs)
