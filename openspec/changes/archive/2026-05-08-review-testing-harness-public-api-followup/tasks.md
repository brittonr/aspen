# Tasks

## 1. API review
- [x] Inventory `aspen-testing` public exports and default feature dependencies.
- [x] Identify reusable inventory/manifest/report helpers that should remain available without adapter-specific runtime stacks.
- [x] Identify heavy adapter crates/features that must remain explicit (`router`, `simulation`, `network`, `jobs`, `ci`, `forge`, `testing`, `full`).

## 2. Implementation
- [x] Add stable structured inventory freshness diagnostics (`InventoryCheckReport`, `InventoryCheckDiagnostic`, severity codes) to the reusable default API.
- [x] Add `aspen-test-harness check --json` for machine-readable freshness diagnostics while preserving existing human output.
- [x] Add a dependency-boundary guard proving the default `aspen-testing --no-default-features` graph excludes VM/network, madsim, runtime-app, forge, CI, jobs, and Raft adapters.
- [x] Expose the boundary guard through `scripts/test-harness.sh public-api-boundary` and include it in the quick confidence rail.

## 3. Documentation and tests
- [x] Document the reusable default API, structured diagnostics, explicit adapter features, and boundary-check command.
- [x] Add positive tests for structured inventory diagnostics and the clean public API boundary.
- [x] Add docs/contract tests for public API boundary documentation and quick-confidence inclusion.
- [x] Run targeted validation, OpenSpec validation, and whitespace checks before archive.
