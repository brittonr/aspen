# Testing Harness Delta: Semantic Nextest Profiles

## ADDED Requirements

### Requirement: Semantic test profiles
r[molten.testing.nextest.semantic_profiles] Molten SHOULD expose semantic test profiles for fast core, harness, CLI, distributed simulation, VM/platform, and dogfood or soak evidence scopes.

#### Scenario: Developer selects smallest useful profile
- GIVEN a change that affects deterministic harness replay behavior
- WHEN a developer inspects the semantic profile matrix
- THEN the matrix identifies the harness-focused command and its expected evidence artifacts before VM or dogfood checks are required

### Requirement: Profile evidence scope is explicit
r[molten.testing.nextest.risk_scope] Each semantic profile MUST declare its evidence scope, command surface, retry policy, expected artifact kinds, cost class, and release-review caveats.

#### Scenario: Distributed simulation does not claim VM evidence
- GIVEN a passing distributed simulation profile run
- WHEN release evidence is summarized
- THEN the profile scope identifies it as deterministic simulation evidence and does not claim VM, live transport, or dogfood readiness evidence

### Requirement: Profile outputs are preserved by Nix checks
r[molten.testing.nextest.nix_outputs] Nix-backed profile checks SHOULD preserve deterministic metadata and rendered JUnit outputs for the selected semantic profile.

#### Scenario: Harness profile emits readback artifacts
- GIVEN the harness semantic profile runs through a Nix check
- WHEN the check succeeds
- THEN the output contains profile metadata, rendered JUnit when configured, and canonical refs or receipts needed for readback

### Requirement: Exploratory retries are excluded from deterministic evidence
r[molten.testing.nextest.exploratory_exclusion] Exploratory profiles MAY allow retries for diagnostics, but retry success MUST NOT satisfy deterministic CI, release, admission, or upgrade evidence gates.

#### Scenario: Retry-only pass remains diagnostic
- GIVEN an exploratory profile passes only after a retry
- WHEN deterministic evidence gates evaluate the run
- THEN the run is excluded as deterministic pass evidence and preserved only as diagnostic evidence
