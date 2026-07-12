## Phase 1: Shared workspace shell

- [x] [serial] Add a dev-only `TestWorkspace` backed by aligned `cap-tempfile` and `cap-std` versions, with RAII lifetime and no caller-selected global temp name. r[molten.testing.cap_std_workspace]
- [x] [serial] Add typed capability-relative subroots and pure logical labels for state, input, output, transport, ledger, cache, and adversarial setup. r[molten.testing.cap_std_subroots]
- [x] [parallel] Add explicit adversarial mutation helpers for symlink, mode, corruption, replacement, and removal setup without exposing that authority to production APIs. r[molten.testing.cap_std_subroots] r[molten.testing.cap_std_validation]

## Phase 2: Shell bridges and retention

- [x] [serial] Add a narrow child-process path bridge that confines spawned CLI state roots to workspace subroots and excludes host paths from canonical evidence. r[molten.testing.cap_std_process_bridge]
- [x] [parallel] Add explicit selected-artifact export and opt-in failure preservation while keeping ordinary cleanup RAII-owned and host paths diagnostic-only. r[molten.testing.cap_std_cleanup] r[molten.testing.cap_std_process_bridge]
- [x] [serial] Remove global temporary-directory prefix scans and stale-path pre-deletion from shared test helpers. r[molten.testing.cap_std_cleanup]

## Phase 3: Representative migration

- [x] [parallel] Migrate shared CLI, chunk, retention, remote dataspace, exchange, and evidence test helpers to `TestWorkspace`. r[molten.testing.cap_std_workspace] r[molten.testing.cap_std_subroots]
- [x] [parallel] Migrate node, async live-transport, cluster, local multiprocess, and process-spawning fixtures to typed subroots plus the explicit process bridge. r[molten.testing.cap_std_process_bridge]
- [x] [serial] Consolidate or remove duplicated pid/counter `temp_dir` helpers in converted scopes. r[molten.testing.cap_std_cleanup]

## Phase 4: Positive, negative, and structural verification

- [x] [parallel] Add positive tests for concurrent isolated workspaces, typed subroots, async fixtures, child-process execution, automatic cleanup, and explicit artifact export. r[molten.testing.cap_std_validation]
- [x] [parallel] Add negative tests for symlink escape, wrong-root substitution, cross-workspace access, cleanup of replaced entries, export denial, and host-path leakage into canonical receipts. r[molten.testing.cap_std_validation]
- [x] [serial] Add scoped ast-grep fixtures and a blocking rule against new process-id/counter temp roots, ambient stale-prefix scans, and broad prefix deletion in converted test helpers. r[molten.testing.cap_std_regression_gate]
- [x] [parallel] Document workspace authority, adversarial setup, process bridges, cleanup limits, artifact retention, and evidence non-claims. r[molten.testing.cap_std_cleanup] r[molten.testing.cap_std_process_bridge]

## Phase 5: Validation

- [x] [serial] Run workspace unit tests and representative store, node, async transport, CLI, cluster, and multiprocess positive and negative suites. r[molten.testing.cap_std_validation] r[molten.testing.cap_std_regression_gate]
- [x] [serial] Run formatting, Clippy, Cairn validation, proposal/design/tasks gates, and relevant nextest/Nix checks before sync and archive. r[molten.testing.cap_std_validation]

Validation evidence: `cargo fmt --all -- --check`, the full `cargo test` suite (1,163 tests), `cargo clippy --all-targets -- -D warnings`, focused Nextest (27 workspace tests), positive/negative and converted-scope ast-grep scans, `nix build path:$PWD#checks.x86_64-linux.cap-std-test-workspaces`, and strict Cairn validation passed on 2026-07-12.
