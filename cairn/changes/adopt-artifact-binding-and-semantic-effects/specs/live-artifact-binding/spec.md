# Artifact Registry Specification Delta

## Purpose

Add explicit one-snapshot late-bound artifact bindings while preserving immutable refs, exact pinning below the resolution boundary, and Molten-owned admission.

## ADDED Requirements

### Requirement: Molten adopts the shared binding core without transferring authority

r[molten.artifacts.live_binding.core_adoption] Molten MUST consume a reviewed immutable `artifact-binding-core` revision only for deterministic binding-transition, snapshot-resolution, reachability, and retirement-classification mechanics. Molten MUST retain Preserves schemas, BLAKE3 artifact identity, registry and ledger storage, atomic mutation, interface compatibility, capability and policy admission, receipts, deployment, retention, and garbage collection.

#### Scenario: Shared core returns a passing transition plan
- GIVEN the shared core validates supplied successor-binding facts
- WHEN Molten evaluates a registry mutation
- THEN Molten MUST still run product compatibility, authority, policy, provenance, resource, lifecycle, and persistence gates before mutation.

#### Scenario: Workspace-relative dependency is proposed for release
- GIVEN a development pilot uses a sibling path
- WHEN release or adoption evidence is prepared
- THEN Molten MUST deny permanent adoption until Cargo and Nix pin the same reviewed immutable repository revision.

### Requirement: Late binding is explicit and unit-scoped

r[molten.artifacts.live_binding.unit_resolution] Molten MUST distinguish exact pinned requests from explicit late-bound service or system-extension requests and MUST resolve a late-bound key exactly once per admitted request, turn, callback pass, job, or protocol session against one immutable binding snapshot.

#### Scenario: New work begins after cutover
- GIVEN binding revision R1 points service S to artifact A and successor R2 points S to artifact B
- WHEN old work already resolved under R1 and new work resolves under R2
- THEN old work MUST remain pinned to A and new work MAY pin B after normal admission.

#### Scenario: Internal call performs implicit re-resolution
- GIVEN a unit of work already pinned an exact target and dependency closure
- WHEN code below that boundary attempts another mutable lookup without an explicit nested late-bound operation
- THEN Molten MUST deny or classify the lookup outside the admitted execution plan.

### Requirement: Binding cutover is load-validate-swap

r[molten.artifacts.live_binding.cutover] A live binding cutover MUST load and verify required artifacts, validate exact expected-current and successor facts, run compatibility and product admission, and only then atomically publish a new binding revision. Failure before publication MUST leave the prior binding current.

#### Scenario: Successor passes every gate
- GIVEN the successor artifact, closure, semantic effects, interfaces, migration requirements, and product evidence pass
- WHEN the compare-and-swap publication succeeds against the expected current revision
- THEN Molten MUST emit a canonical binding transition receipt naming prior and successor revisions and targets.

#### Scenario: Current revision changed concurrently
- GIVEN another publication advanced the binding after planning
- WHEN the stale plan attempts publication
- THEN mutation MUST fail closed and diagnostics MUST name expected and observed revisions without replacing the current binding.

### Requirement: Live bindings remain non-authority metadata

r[molten.artifacts.live_binding.non_authority] A binding key, current target, revision, resolution receipt, or trusted-looking service name MUST NOT grant execution, capability, policy, provenance, source-gate, resource, transport, retention, deployment, or release authority.

#### Scenario: Binding resolves but execution evidence is absent
- GIVEN a late-bound request resolves one exact artifact ref
- WHEN normal runtime admission lacks required evidence
- THEN execution MUST deny even though resolution succeeded.
