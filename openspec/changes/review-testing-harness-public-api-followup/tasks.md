## Phase 1: API Review

- [x] [serial] Create the OpenSpec baseline for the testing harness public API follow-up.
- [ ] [serial] Inventory current `aspen-testing` exports, feature flags, and dependency graph for reusable versus adapter-specific surfaces.

## Phase 2: Boundary Fixes

- [ ] [depends:inventory] Add negative dependency checks or fixtures for reusable defaults versus VM/patchbay/madsim/network/runtime adapters.
- [ ] [depends:negative-checks] Tighten or document public APIs so inventory/report helpers remain reusable and adapter helpers are explicit.
- [ ] [depends:api-fixes] Update runtime-host readiness checks to use structured diagnostics where practical.

## Phase 3: Verification

- [ ] [depends:readiness-checks] Run `cargo test -p aspen-testing`, harness export/check, dependency policy checks, OpenSpec validation, and whitespace checks.
