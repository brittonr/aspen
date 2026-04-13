## Phase 1: CI executor and federation monoliths

- [x] Refactor `crates/aspen-ci-executor-nix/src/flake_lock.rs:deserialize`, `crates/aspen-ci-executor-nix/src/executor.rs:try_native_build`, and `crates/aspen-ci-executor-nix/src/executor.rs:execute_build` into bounded helpers with characterization coverage.
- [x] Refactor `crates/aspen-forge-handler/src/handler/handlers/federation.rs:{federation_import_objects, handle_federation_fetch_refs, handle_federation_bidi_sync}` and `crates/aspen-forge-handler/src/handler/handlers/federation_git.rs:sync_from_origin` into smaller phases with explicit invariants/assertions.

## Phase 2: Dispatch and lookup-table cleanup

- [x] Replace the hand-written mega-match logic in `crates/aspen-client-api/src/messages/mod.rs:{variant_name, required_app}` with a generated or table-driven mapping plus regression coverage.
- [x] Split `crates/aspen-cluster/src/gossip/discovery/lifecycle.rs:start_internal`, `crates/aspen-transport/src/log_subscriber/connection.rs:handle_log_subscriber_connection`, and `crates/aspen-core-essentials-handler/src/core.rs:handle_alert_evaluate` into bounded orchestration helpers.

## Phase 3: Trust and secrets rotation regressions

- [x] Extract deterministic planning helpers from `crates/aspen-raft/src/node/trust.rs:{collect_old_shares_for_reconfiguration, probe_for_peer_expungement, rotate_trust_after_membership_change}` and add assertions around epoch/threshold invariants.
- [x] Refactor `crates/aspen-raft/src/secrets_at_rest.rs:collect_shares` and `crates/aspen-cluster/src/bootstrap/node/storage_init.rs:create_raft_instance` to reduce recent function-length regressions introduced by trust/secrets work.

## Phase 4: API-size leaks and scanner quality

- [x] Remove the highest-priority production `usize_api` leaks in `crates/aspen-jobs/src/scheduler.rs:recover_schedules`, `crates/aspen-forge/src/git/bridge/exporter.rs:export_commit_dag_blake3`, and `crates/aspen-jobs/src/distributed_pool.rs:create_worker_group`.
- [x] Improve `scripts/tigerstyle-audit.py` so doc comments do not trigger `verified_ambient_time` and `crates/aspen-trust/src/key_manager.rs:get_if_initialized` no longer reports a parse error; save fixture/transcript evidence with that remediation change.
