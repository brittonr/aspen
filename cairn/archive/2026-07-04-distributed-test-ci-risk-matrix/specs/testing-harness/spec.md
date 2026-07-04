## ADDED Requirements

### Requirement: Distributed CI profiles are explicit
r[molten.testing.distributed_ci.profile_matrix] Molten MUST define an explicit distributed test risk/cost matrix covering fast core checks, deterministic protocol simulation, CLI receipt workflows, VM smoke checks, executable VM fault checks, and soak or pilot evidence. Each matrix entry MUST name its command surface, expected artifact kinds, evidence scope, cost class, and release-review status.

#### Scenario: Release reviewer sees distributed test scope
- GIVEN the distributed CI matrix is rendered for a candidate tree
- WHEN a reviewer inspects the matrix
- THEN each profile identifies the command to run, authoritative receipt artifacts, unsupported or unavailable states, and claims that remain out of scope.

### Requirement: Distributed test metadata is bound canonically
r[molten.testing.distributed_ci.metadata_binding] Distributed test run evidence MUST bind source or tree refs, Nix input refs where applicable, test binary or package refs, profile and shard refs, seed refs, topology refs, fault-plan refs, emitted receipt refs, allowed variance declarations, and diagnostic log refs.

#### Scenario: Shard evidence is reproducible
- GIVEN a distributed test shard emits pass evidence
- WHEN the shard metadata is parsed
- THEN it identifies the source, Nix inputs, binary, profile, seed, topology, fault plan, child receipts, and declared variance needed to reproduce or audit the shard.

### Requirement: Traceability is required for distributed evidence gates
r[molten.testing.distributed_ci.traceability_required_gate] Release or CI review for distributed evidence-bearing requirements MUST require traceability coverage that includes positive evidence, negative evidence, validation commands, and artifact refs, or an explicit documented exemption. Missing, stale, or unsupported coverage MUST deny the distributed evidence gate.

#### Scenario: Missing negative coverage denies release evidence
- GIVEN a distributed requirement has positive VM evidence but no negative denial or exemption evidence
- WHEN the distributed CI traceability gate runs
- THEN the gate denies or marks the requirement incomplete before release evidence can pass.

### Requirement: Retry success is not pass evidence
r[molten.testing.distributed_ci.retry_policy] Distributed CI and release profiles that emit pass evidence MUST run with zero retries or otherwise bind every attempted run and deny retry-only success as proof of deterministic behavior. Exploratory reruns MAY produce diagnostic or quarantine evidence but MUST NOT satisfy pass gates without explicit review evidence.

#### Scenario: Flaky test passes only after retry
- GIVEN a distributed test fails on the first attempt and passes on a retry
- WHEN release evidence evaluates the run
- THEN the run is not accepted as deterministic pass evidence
- AND diagnostics identify the failed attempt and retry boundary.

### Requirement: Unsupported distributed profiles are unavailable, not passing
r[molten.testing.distributed_ci.unavailable_handling] Distributed CI profiles requiring VM, network, live transport, or soak support MUST record unavailable, skipped, or denied evidence when required support is absent. Unsupported execution MUST NOT be treated as a passing profile.

#### Scenario: VM fault profile unavailable in CI
- GIVEN the CI host lacks support for executable VM fault injection
- WHEN the distributed CI matrix evaluates `vm-fault`
- THEN the matrix records unavailable evidence for that profile
- AND any broader gate either excludes it by explicit policy or denies claims requiring it.

### Requirement: Distributed CI gates have negative fixtures
r[molten.testing.distributed_ci.negative_fixtures] Molten SHOULD test distributed CI matrix validation with negative fixtures for missing shard artifacts, missing positive coverage, missing negative coverage, stale requirement refs, retry-only success, skipped VM support, and undeclared variance.

#### Scenario: Stale traceability ref fails
- GIVEN a traceability manifest entry points to a requirement id that no longer exists or a command that no longer produces the referenced artifact
- WHEN the distributed CI gate evaluates the manifest
- THEN validation fails closed with stale-reference diagnostics.

### Requirement: Distributed CI matrix is documented
r[molten.testing.distributed_ci.docs] User-facing documentation SHOULD describe distributed test profiles, commands, expected artifacts, reproducibility metadata, traceability gates, retry policy, unavailable handling, and evidence-only boundaries.

#### Scenario: Developer picks the right shard
- GIVEN a developer is changing distributed protocol logic
- WHEN they read the distributed testing docs
- THEN they can identify the smallest relevant profile to run before VM or soak checks and the evidence expected for release review.
