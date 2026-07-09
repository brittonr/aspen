# Testing Harness

## ADDED Requirements

### Requirement: Nextest profiles declare semantic partitions

r[molten.testing.nextest_profiles.semantic_partitions] Molten SHOULD maintain reviewed semantic rows for Nextest profiles that bind profile id, filter selection, expected artifacts, retry policy, evidence scope, cost class, and platform availability.

#### Scenario: Fast-core profile selects pure fast tests

- GIVEN the `fast-core` profile is intended for pure core, parser, and receipt unit coverage
- WHEN profile validation reads the semantic row and Nextest config
- THEN the profile has a filter or metadata contract that selects only the reviewed fast-core test partition.

#### Scenario: VM profile records platform caveat

- GIVEN the `vm-platform` profile depends on platform VM support
- WHEN profile validation reads the semantic row
- THEN the row records platform availability requirements and release caveats rather than treating VM success as universal deterministic evidence.

### Requirement: Deterministic profiles exclude non-replayable tests

r[molten.testing.nextest_profiles.deterministic_exclusion] Deterministic evidence profiles MUST exclude live-only, VM-only, exploratory, retry-only, or diagnostic-only tests unless those tests are explicitly classified as excluded from deterministic pass evidence.

#### Scenario: Live-only test is excluded from deterministic profile

- GIVEN a test is marked live-only or non-replayable
- WHEN `fast-core`, `harness`, `cli`, or `distributed-simulation` profile validation runs
- THEN validation denies if the deterministic profile includes the test as pass evidence.

#### Scenario: Exploratory retry does not satisfy deterministic evidence

- GIVEN an exploratory profile retries a flaky test and eventually reports pass
- WHEN deterministic readiness or release evidence is computed
- THEN retry-only success is classified as diagnostic and cannot satisfy deterministic pass evidence.

### Requirement: Nextest config readback binds filters and artifacts

r[molten.testing.nextest_profiles.config_readback] The Nextest configuration readback SHOULD preserve profile filters or metadata selectors, retry policy, expected JUnit/artifact paths, command surfaces, and validation diagnostics as review evidence.

#### Scenario: Profile readback preserves expected artifacts

- GIVEN the `ci` profile has an expected JUnit path and command surface
- WHEN the nextest-config check runs
- THEN the check output includes the profile readback, command surface, expected JUnit path, and validation result.

#### Scenario: Missing filter denies profile readback

- GIVEN a semantic profile row requires a filter or metadata selector
- WHEN the Nextest config lacks that selector
- THEN profile validation denies with a diagnostic naming the profile and missing selector.

### Requirement: Nextest profile behavior has positive and negative coverage

r[molten.testing.nextest_profiles.positive_negative_coverage] Nextest profile validation SHOULD include positive tests for valid profile rows and negative tests for missing filters, duplicate profile ids, deterministic/live mixing, unsupported retry policy, missing artifacts, and JUnit-only evidence misuse.

#### Scenario: Valid profile matrix passes

- GIVEN every reviewed profile row has a supported filter, retry policy, artifact path, evidence scope, and platform classification
- WHEN profile matrix validation runs
- THEN it passes and emits review diagnostics for each profile.

#### Scenario: JUnit-only evidence is rejected

- GIVEN a profile readback includes a JUnit path but lacks the required command surface, test metadata, or canonical test-run receipt binding
- WHEN profile validation runs for release evidence
- THEN validation denies before JUnit output can be treated as canonical pass evidence.
