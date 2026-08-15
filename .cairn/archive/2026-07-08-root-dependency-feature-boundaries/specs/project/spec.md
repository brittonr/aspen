# Project Delta: Root Dependency and Feature Boundaries

### Requirement: Root dependencies are classified by layer
r[molten.project.modularity.dependency_classes] Cargo dependencies SHOULD be classified by their intended layer: core, codec, policy-evidence, runtime, adapter, CLI, test, or integration.

#### Scenario: Dependency purpose is reviewable
- GIVEN a dependency in the repository manifest
- WHEN reviewers inspect dependency classification
- THEN the dependency has an intended layer and migration status

### Requirement: Minimal core build excludes adapters
r[molten.project.modularity.minimal_core_build] The project SHOULD provide a minimal core or core-plus-codec build surface that excludes transport, storage, executor, CLI, live policy tooling, and integration dependencies not required for pure validation.

#### Scenario: Minimal core compiles without adapters
- GIVEN the minimal core build target
- WHEN focused validation builds it
- THEN it succeeds without requiring Iroh, Redb, Wasmtime, Steel execution, Nickel CLI/tooling, NixOS VM, dogfood, or live transport dependencies

#### Scenario: Adapter leak is reported
- GIVEN the minimal core surface imports an adapter dependency
- WHEN dependency validation runs
- THEN validation fails or reports the adapter leak before release evidence is promoted

### Requirement: Default build compatibility is preserved
r[molten.project.modularity.default_compatibility] Introducing dependency or feature boundaries MUST preserve existing default developer build and CLI behavior unless a separate compatibility change owns the break.

#### Scenario: Default build still includes integrations
- GIVEN a developer runs the existing default build or CLI test path
- WHEN feature boundaries are introduced
- THEN the default path continues to include required adapters and integration features

### Requirement: Dependency-boundary checks include positive and negative coverage
r[molten.project.modularity.dependency_tests] Dependency-boundary changes SHOULD include positive checks for the intended minimal surface and negative checks or diagnostics for forbidden adapter leakage.

#### Scenario: Dependency checks cover leak
- GIVEN a forbidden adapter import appears in the minimal core surface
- WHEN the dependency check runs
- THEN it reports the offending dependency and owning layer
