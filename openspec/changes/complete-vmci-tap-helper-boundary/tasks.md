## Phase 1: Contract

- [x] [serial] Add VM-CI TAP helper boundary delta spec with helper/readiness scenarios.
- [x] [serial] Validate proposal/design/tasks/spec gate before implementation.

## Phase 2: Implementation

- [x] [serial] Add `aspen-tap-helper` binary with allowlisted `ensure`/`delete` operations.
- [x] [serial] Enable `NetworkMode::TapWithHelper` validation and runtime use of helper for ensure/delete.
- [x] [serial] Update setup/dogfood wiring so setup installs the helper and dogfood defaults to helper mode when available.
- [x] [serial] Update VM-CI readiness diagnostics for helper-backed TAP mode.

## Phase 3: Verification

- [x] [serial] Run focused unit/config/helper/readiness tests.
- [x] [serial] Run OpenSpec strict validation and whitespace checks.
- [ ] [serial] Rerun live VM-CI dogfood acceptance or record the highest verified host boundary.
- [ ] [serial] Sync/archive the OpenSpec when all implementation and verification tasks are complete.
