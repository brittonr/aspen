# Tasks: semantic-nextest-profiles

## Profile model

- [x] [serial] r[molten.testing.nextest.semantic_profiles] Define semantic test profile metadata for fast core, harness, CLI, distributed simulation, VM/platform, and dogfood/soak scopes.
- [x] [parallel] r[molten.testing.nextest.risk_scope] Bind each profile to evidence scope, command surface, expected artifacts, retry policy, cost class, and release-review caveats.

## Execution wiring

- [x] [serial] r[molten.testing.nextest.nix_outputs] Wire profile commands through nextest and Nix checks so each profile emits deterministic metadata and rendered JUnit where applicable.
- [x] [parallel] r[molten.testing.nextest.exploratory_exclusion] Ensure exploratory retry success cannot satisfy deterministic CI, release, admission, or upgrade evidence gates.

## Tests and docs

- [x] [parallel] r[molten.testing.nextest.semantic_profiles] Add positive fixtures for valid profile metadata and negative fixtures for missing profile id, mismatched command surface, unsupported retry policy, unavailable required platform, and stale artifact kind.
- [x] [serial] r[molten.testing.nextest.risk_scope] Update README or distributed testing docs with the semantic command matrix and evidence-only boundaries.
- [x] [serial] r[molten.testing.nextest.nix_outputs] Run nextest config validation, focused profile tests, and Cairn validation; record blockers if any profile cannot run locally.
