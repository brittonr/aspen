# Project Delta: Core Crate Boundary

### Requirement: Pure core crate boundary
r[molten.modularity.core_crate.pure_foundation] The repository SHOULD provide a dedicated core crate for foundational deterministic types and pure validation that can be tested without adapters, CLI commands, filesystem state, network services, clocks, or process execution.

#### Scenario: Core validator runs in memory
- GIVEN a core validation API for refs, envelopes, bounds, or identity inputs
- WHEN a unit test calls the API with in-memory valid data
- THEN the API returns a structured pass result without reading files, spawning processes, opening network connections, reading clocks, or rendering CLI output

#### Scenario: Core rejects malformed data before adapters
- GIVEN malformed refs, missing required fields, invalid bounds, or unsupported states
- WHEN a core validation API evaluates the input
- THEN it returns a structured error or deny result before any adapter or CLI shell is invoked

### Requirement: Core dependency direction is enforced
r[molten.modularity.core_crate.dependency_direction] The core crate MUST NOT depend on adapter crates, CLI modules, filesystem traversal, process execution, environment reads, wall-clock reads, Iroh, Redb, Wasmtime, Steel execution, or live Nickel evaluation.

#### Scenario: Adapter dependency is blocked
- GIVEN a proposed core crate change imports an adapter or CLI dependency
- WHEN dependency-boundary validation runs
- THEN validation fails or records the violation before release evidence is promoted

#### Scenario: Root crate re-export preserves compatibility
- GIVEN a foundational item moves into the core crate
- WHEN existing callers use the previous root-crate module path during the migration window
- THEN compatibility re-exports continue to compile until a separate public API change removes them

### Requirement: Core extraction carries positive and negative evidence
r[molten.modularity.core_crate.validation] Core extraction changes SHOULD include positive and negative tests or fixtures for each moved invariant, or record an explicit exemption when the moved surface is a re-export only.

#### Scenario: Positive and negative moved invariant tests exist
- GIVEN a moved core invariant is executable
- WHEN reviewers inspect the change evidence
- THEN valid examples and invalid examples are both covered by focused tests or fixtures
