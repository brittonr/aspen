# Tasks: semantic-nextest-profile-rails

## Phase 1: Profile manifest and pure validation

- [x] [serial] r[molten.testing.nextest_profiles.semantic_partitions] Define reviewed semantic rows for each Nextest profile, including filter selection, expected artifacts, retry policy, evidence scope, cost class, and platform availability.
- [x] [serial] r[molten.testing.nextest_profiles.config_readback] Add a pure validation core for profile rows, duplicate profile ids, missing filters, retry policy, expected artifacts, and platform flags.
- [x] [parallel] r[molten.testing.nextest_profiles.deterministic_exclusion] Add non-replayable/live/VM/exploratory exclusion checks for deterministic evidence profiles.

## Phase 2: Shell readback and config changes

- [x] [parallel] r[molten.testing.nextest_profiles.semantic_partitions] Add or update Nextest filter expressions/test metadata so each profile runs the suites named by its semantic row.
- [x] [parallel] r[molten.testing.nextest_profiles.config_readback] Extend the Nix `nextest-config` check/readback to preserve profile filters, JUnit paths, retry behavior, and validation diagnostics.

## Phase 3: Positive/negative evidence and docs

- [x] [serial] r[molten.testing.nextest_profiles.positive_negative_coverage] Add positive profile validation tests and negative tests for missing filters, deterministic/live mixing, duplicate profile rows, unsupported retry policy, missing artifacts, and JUnit-only evidence misuse.
- [x] [serial] r[molten.testing.nextest_profiles.deterministic_exclusion] Update README/proof workflow docs to explain which profiles can satisfy deterministic or release evidence and which are diagnostic-only.
- [x] [serial] r[molten.testing.nextest_profiles.positive_negative_coverage] Run focused testing-hardening tests, `nix build .#checks.$system.nextest-config --no-link`, and Cairn validation/gates.
