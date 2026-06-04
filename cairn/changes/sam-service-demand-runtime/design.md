## Context

Molten already has a local runtime kernel, deterministic harness reports, authority/capability/resource/effect-handle evidence, and remote dataspace envelopes. The service demand runtime should be a thin SAM layer over those primitives: demand and readiness are dataspace facts, startup is an admitted turn, and all side effects are receipt-bound.

This change depends on canonical service records from `sam-service-records-ledger` and refines the broader `sam-service-supervision-runtime` roadmap.

## Goals

- Evaluate `service-demand-v1` assertions against known `service-manifest-v1` records.
- Resolve dependency readiness from canonical service status/readiness assertions.
- Gate startup through explicit authority, policy, resource, effect-handle, and strict source-gate evidence before actor execution.
- Commit service-owned readiness, degraded, failure, and stopped assertions through the local dataspace runtime.
- Emit `service-lifecycle-receipt-v1` values for start, deny, ready, fail, stop, and dependency-wait decisions.
- Bind replay identity from demand refs, manifest refs, dependency refs, authority/resource/effect refs, source-gate refs, scheduler profile, and recorded effect log refs.
- Provide a deterministic two-service harness/CLI fixture.

## Non-Goals

- No OS process spawning or systemd/Kubernetes compatibility.
- No unbounded service graph resolution.
- No remote discovery semantics beyond existing envelope compatibility.
- No restart policy loops or supervision trees beyond single transition receipts; full restart cleanup is a follow-up Cairn.
- No implicit authority from service ids, actor ids, or local filesystem paths.

## Demand Evaluation

1. Read bounded service manifests and demand assertions from the local dataspace/fixture state.
2. Match demand to a manifest by canonical service id and manifest ref.
3. Check dependency readiness assertions for each required service id/ref.
4. Build startup admission input from manifest owner authority, policy refs, resource refs, effect profile refs, and source-gate refs.
5. Deny before actor execution if any gate fails or if dependencies are not ready.
6. Execute the admitted service start inside a runtime turn and commit owned readiness/status facts.
7. Emit lifecycle receipt refs and turn-journal context refs for replay and gate checks.

## Replay Identity

Replay identity MUST include:

- service manifest ref;
- demand assertion ref;
- dependency readiness/status refs;
- authority, policy, capability, resource, effect-handle, and source-gate refs;
- scheduler profile/ref and bounded logical time seed;
- recorded effect log refs for actor startup outputs;
- prior service status ref if the service is already running or stopped.

Replay fails at first divergence in admission decision, dependency wait/ready state, lifecycle receipt ref, owned assertion set, or effect-log binding.

## CLI/Test Shape

A local test command may use a shape like:

```sh
cargo run -- test service run-two-service \
  --suite examples/service-two-stage.preserves \
  --out target/molten-service/two-service
```

The command should produce canonical service demand/status/lifecycle receipt artifacts and a summary view. The summary is non-normative; tests and gates parse the Preserves artifacts.

## Denial Cases

- Missing manifest, malformed manifest, or unknown schema tag.
- Missing startup authority, policy, resource, effect-handle, or source-gate evidence.
- Dependency not ready or dependency status ref stale/tampered.
- Actor start attempts to emit readiness outside its owned assertion namespace.
- Runtime tries to reuse source-side state instead of target/local service state.
- Replay identity omits demand, dependency, authority/resource/effect, scheduler, or effect-log refs.
