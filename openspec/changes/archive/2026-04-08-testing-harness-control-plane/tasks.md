## 1. Nickel Suite Metadata

- [x] 1.1 Define a Nickel schema for test suite manifests, including suite ID, layer, owner, runtime class, prerequisites, tags, and execution target fields.
- [x] 1.2 Add a manifest directory and author initial Nickel manifests for representative Rust integration, patchbay, and NixOS VM suites.
- [x] 1.3 Add a validation/export command that fails on invalid or stale suite metadata.

## 2. Generated Inventory and Selection

- [x] 2.1 Generate a normalized machine-consumable inventory from the Nickel manifests for Rust and Nix consumers.
- [x] 2.2 Wire `flake.nix` VM or long-running suite registration to generated inventory data instead of a hand-maintained per-suite block.
- [x] 2.3 Update focused test selection entry points (`nextest` helpers, test scripts, or equivalent commands) to resolve grouping from generated inventory data.
- [x] 2.4 Register patchbay suites in the shared metadata and generated inventory path.

## 3. Shared Harness Facade

- [x] 3.1 Add shared façade types or traits under `crates/aspen-testing*` for cluster bootstrap, client access, readiness waits, and layer-specific adapters.
- [x] 3.2 Migrate the real-cluster bootstrap logic from `tests/support/real_cluster.rs` into the shared testing crates or reduce it to a compatibility wrapper.
- [x] 3.3 Update the affected Rust integration suites to use the shared harness facade.

## 4. Wait-Driven Test Flows

- [x] 4.1 Add bounded Rust wait helpers for leader election, replication, cluster health, and job completion conditions.
- [x] 4.2 Add shared NixOS VM readiness helpers under `nix/tests/lib/` for service, socket, cluster, and job waits.
- [x] 4.3 Replace fixed-sleep readiness checks in the highest-value Rust and VM suites with the new wait helpers.

## 5. Harness Reporting

- [x] 5.1 Capture retry counts, retry-only passes, skips, timeout counts, and duration data into a machine-readable run report.
- [x] 5.2 Generate a coverage-by-layer summary from suite metadata and run outputs.
- [x] 5.3 Expose harness reports through local validation commands and at least one CI-oriented path.

## 6. Verification and Rollout

- [x] 6.1 Add regression coverage for metadata validation, generated inventory consumers, shared harness APIs, and reporting output.
- [x] 6.2 Run focused validation for the migrated harness layers and record the expected commands in repo documentation.
- [x] 6.3 Update `AGENTS.md` and `.agent/napkin.md` with any non-obvious harness workflow changes discovered during implementation.
