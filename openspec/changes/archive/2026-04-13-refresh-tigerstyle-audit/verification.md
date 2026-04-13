# Verification Evidence

## Implementation Evidence

- Changed file: `crates/aspen-ci-executor-nix/src/executor.rs`
- Changed file: `crates/aspen-ci-executor-nix/src/flake_lock.rs`
- Changed file: `crates/aspen-client-api/src/lib.rs`
- Changed file: `crates/aspen-client-api/src/messages/mod.rs`
- Changed file: `crates/aspen-client-api/src/messages/request_metadata.rs`
- Changed file: `crates/aspen-cluster/src/bootstrap/node/storage_init.rs`
- Changed file: `crates/aspen-cluster/src/gossip/discovery/lifecycle.rs`
- Changed file: `crates/aspen-cluster/src/gossip/discovery/mod.rs`
- Changed file: `crates/aspen-core-essentials-handler/src/core.rs`
- Changed file: `crates/aspen-forge-handler/src/handler/handlers/federation.rs`
- Changed file: `crates/aspen-forge-handler/src/handler/handlers/federation_git.rs`
- Changed file: `crates/aspen-forge/src/git/bridge/exporter.rs`
- Changed file: `crates/aspen-forge/src/resolver.rs`
- Changed file: `crates/aspen-jobs/src/distributed_pool.rs`
- Changed file: `crates/aspen-jobs/src/scheduler.rs`
- Changed file: `crates/aspen-raft/src/node/trust.rs`
- Changed file: `crates/aspen-raft/src/secrets_at_rest.rs`
- Changed file: `crates/aspen-transport/src/log_subscriber/connection.rs`
- Changed file: `scripts/tigerstyle-audit.py`
- Changed file: `tools/tigerstyle/test_tigerstyle_audit.py`
- Changed file: `openspec/changes/refresh-tigerstyle-audit/audit-report.md`
- Changed file: `openspec/changes/refresh-tigerstyle-audit/design.md`
- Changed file: `openspec/changes/refresh-tigerstyle-audit/proposal.md`
- Changed file: `openspec/changes/refresh-tigerstyle-audit/specs/tigerstyle-audit/spec.md`
- Changed file: `openspec/changes/refresh-tigerstyle-audit/tasks.md`
- Changed file: `openspec/changes/refresh-tigerstyle-audit/verification.md`
- Changed file: `openspec/changes/refresh-tigerstyle-audit/evidence/aspen-ci-executor-nix-detect-project-type.txt`
- Changed file: `openspec/changes/refresh-tigerstyle-audit/evidence/aspen-ci-executor-nix-find-missing-store-paths.txt`
- Changed file: `openspec/changes/refresh-tigerstyle-audit/evidence/aspen-client-api-request-metadata.txt`
- Changed file: `openspec/changes/refresh-tigerstyle-audit/evidence/aspen-cluster-start-internal.txt`
- Changed file: `openspec/changes/refresh-tigerstyle-audit/evidence/aspen-core-essentials-handler-alert-evaluate.txt`
- Changed file: `openspec/changes/refresh-tigerstyle-audit/evidence/aspen-forge-federation-export.txt`
- Changed file: `openspec/changes/refresh-tigerstyle-audit/evidence/aspen-forge-handler-prepare-objects.txt`
- Changed file: `openspec/changes/refresh-tigerstyle-audit/evidence/aspen-forge-handler-update-have-hashes.txt`
- Changed file: `openspec/changes/refresh-tigerstyle-audit/evidence/aspen-raft-required-peer-confirmations.txt`
- Changed file: `openspec/changes/refresh-tigerstyle-audit/evidence/aspen-raft-secrets-at-rest.txt`
- Changed file: `openspec/changes/refresh-tigerstyle-audit/evidence/aspen-raft-share-collection-target.txt`
- Changed file: `openspec/changes/refresh-tigerstyle-audit/evidence/aspen-transport-log-subscriber.txt`
- Changed file: `openspec/changes/refresh-tigerstyle-audit/evidence/cargo-check-targeted-packages.txt`
- Changed file: `openspec/changes/refresh-tigerstyle-audit/evidence/direct-hotspot-verification.json`
- Changed file: `openspec/changes/refresh-tigerstyle-audit/evidence/direct-hotspot-verification.txt`
- Changed file: `openspec/changes/refresh-tigerstyle-audit/evidence/git-status.txt`
- Changed file: `openspec/changes/refresh-tigerstyle-audit/evidence/implementation-diff.txt`
- Changed file: `openspec/changes/refresh-tigerstyle-audit/evidence/openspec-preflight.txt`
- Changed file: `openspec/changes/refresh-tigerstyle-audit/evidence/scanner-regression-tests.txt`
- Changed file: `openspec/changes/refresh-tigerstyle-audit/evidence/tigerstyle-targeted-scan.json`
- Changed file: `openspec/changes/refresh-tigerstyle-audit/evidence/tigerstyle-targeted-scan.txt`

## Task Coverage

- [x] Refactor `crates/aspen-ci-executor-nix/src/flake_lock.rs:deserialize`, `crates/aspen-ci-executor-nix/src/executor.rs:try_native_build`, and `crates/aspen-ci-executor-nix/src/executor.rs:execute_build` into bounded helpers with characterization coverage.
  - Evidence: `crates/aspen-ci-executor-nix/src/flake_lock.rs`, `crates/aspen-ci-executor-nix/src/executor.rs`, `openspec/changes/refresh-tigerstyle-audit/evidence/aspen-ci-executor-nix-detect-project-type.txt`, `openspec/changes/refresh-tigerstyle-audit/evidence/aspen-ci-executor-nix-find-missing-store-paths.txt`, `openspec/changes/refresh-tigerstyle-audit/evidence/direct-hotspot-verification.txt`, `openspec/changes/refresh-tigerstyle-audit/evidence/tigerstyle-targeted-scan.txt`, `openspec/changes/refresh-tigerstyle-audit/evidence/implementation-diff.txt`
- [x] Refactor `crates/aspen-forge-handler/src/handler/handlers/federation.rs:{federation_import_objects, handle_federation_fetch_refs, handle_federation_bidi_sync}` and `crates/aspen-forge-handler/src/handler/handlers/federation_git.rs:sync_from_origin` into smaller phases with explicit invariants/assertions.
  - Evidence: `crates/aspen-forge-handler/src/handler/handlers/federation.rs`, `crates/aspen-forge-handler/src/handler/handlers/federation_git.rs`, `crates/aspen-forge/src/resolver.rs`, `openspec/changes/refresh-tigerstyle-audit/evidence/aspen-forge-handler-prepare-objects.txt`, `openspec/changes/refresh-tigerstyle-audit/evidence/aspen-forge-handler-update-have-hashes.txt`, `openspec/changes/refresh-tigerstyle-audit/evidence/tigerstyle-targeted-scan.txt`, `openspec/changes/refresh-tigerstyle-audit/evidence/implementation-diff.txt`
- [x] Replace the hand-written mega-match logic in `crates/aspen-client-api/src/messages/mod.rs:{variant_name, required_app}` with a generated or table-driven mapping plus regression coverage.
  - Evidence: `crates/aspen-client-api/src/messages/mod.rs`, `crates/aspen-client-api/src/messages/request_metadata.rs`, `crates/aspen-client-api/src/lib.rs`, `openspec/changes/refresh-tigerstyle-audit/evidence/aspen-client-api-request-metadata.txt`, `openspec/changes/refresh-tigerstyle-audit/evidence/tigerstyle-targeted-scan.txt`, `openspec/changes/refresh-tigerstyle-audit/evidence/implementation-diff.txt`
- [x] Split `crates/aspen-cluster/src/gossip/discovery/lifecycle.rs:start_internal`, `crates/aspen-transport/src/log_subscriber/connection.rs:handle_log_subscriber_connection`, and `crates/aspen-core-essentials-handler/src/core.rs:handle_alert_evaluate` into bounded orchestration helpers.
  - Evidence: `crates/aspen-cluster/src/gossip/discovery/lifecycle.rs`, `crates/aspen-cluster/src/gossip/discovery/mod.rs`, `crates/aspen-transport/src/log_subscriber/connection.rs`, `crates/aspen-core-essentials-handler/src/core.rs`, `openspec/changes/refresh-tigerstyle-audit/evidence/aspen-cluster-start-internal.txt`, `openspec/changes/refresh-tigerstyle-audit/evidence/aspen-transport-log-subscriber.txt`, `openspec/changes/refresh-tigerstyle-audit/evidence/aspen-core-essentials-handler-alert-evaluate.txt`, `openspec/changes/refresh-tigerstyle-audit/evidence/direct-hotspot-verification.txt`, `openspec/changes/refresh-tigerstyle-audit/evidence/implementation-diff.txt`
- [x] Extract deterministic planning helpers from `crates/aspen-raft/src/node/trust.rs:{collect_old_shares_for_reconfiguration, probe_for_peer_expungement, rotate_trust_after_membership_change}` and add assertions around epoch/threshold invariants.
  - Evidence: `crates/aspen-raft/src/node/trust.rs`, `openspec/changes/refresh-tigerstyle-audit/evidence/aspen-raft-required-peer-confirmations.txt`, `openspec/changes/refresh-tigerstyle-audit/evidence/aspen-raft-share-collection-target.txt`, `openspec/changes/refresh-tigerstyle-audit/evidence/tigerstyle-targeted-scan.txt`, `openspec/changes/refresh-tigerstyle-audit/evidence/implementation-diff.txt`
- [x] Refactor `crates/aspen-raft/src/secrets_at_rest.rs:collect_shares` and `crates/aspen-cluster/src/bootstrap/node/storage_init.rs:create_raft_instance` to reduce recent function-length regressions introduced by trust/secrets work.
  - Evidence: `crates/aspen-raft/src/secrets_at_rest.rs`, `crates/aspen-cluster/src/bootstrap/node/storage_init.rs`, `openspec/changes/refresh-tigerstyle-audit/evidence/aspen-raft-secrets-at-rest.txt`, `openspec/changes/refresh-tigerstyle-audit/evidence/direct-hotspot-verification.txt`, `openspec/changes/refresh-tigerstyle-audit/evidence/cargo-check-targeted-packages.txt`, `openspec/changes/refresh-tigerstyle-audit/evidence/implementation-diff.txt`
- [x] Remove the highest-priority production `usize_api` leaks in `crates/aspen-jobs/src/scheduler.rs:recover_schedules`, `crates/aspen-forge/src/git/bridge/exporter.rs:export_commit_dag_blake3`, and `crates/aspen-jobs/src/distributed_pool.rs:create_worker_group`.
  - Evidence: `crates/aspen-jobs/src/scheduler.rs`, `crates/aspen-jobs/src/distributed_pool.rs`, `crates/aspen-forge/src/git/bridge/exporter.rs`, `openspec/changes/refresh-tigerstyle-audit/evidence/aspen-forge-federation-export.txt`, `openspec/changes/refresh-tigerstyle-audit/evidence/tigerstyle-targeted-scan.txt`, `openspec/changes/refresh-tigerstyle-audit/evidence/implementation-diff.txt`
- [x] Improve `scripts/tigerstyle-audit.py` so doc comments do not trigger `verified_ambient_time` and `crates/aspen-trust/src/key_manager.rs:get_if_initialized` no longer reports a parse error; save fixture/transcript evidence with that remediation change.
  - Evidence: `scripts/tigerstyle-audit.py`, `tools/tigerstyle/test_tigerstyle_audit.py`, `crates/aspen-trust/src/key_manager.rs`, `openspec/changes/refresh-tigerstyle-audit/evidence/scanner-regression-tests.txt`, `openspec/changes/refresh-tigerstyle-audit/evidence/tigerstyle-targeted-scan.txt`, `openspec/changes/refresh-tigerstyle-audit/evidence/implementation-diff.txt`

## Review Scope Snapshot

### `git diff HEAD -- remediation targets`

- Status: captured
- Artifact: `openspec/changes/refresh-tigerstyle-audit/evidence/implementation-diff.txt`

## Verification Commands

### `python3 -m unittest tools/tigerstyle/test_tigerstyle_audit.py`

- Status: pass
- Artifact: `openspec/changes/refresh-tigerstyle-audit/evidence/scanner-regression-tests.txt`

### `cargo test -p aspen-client-api test_request_metadata_lookup_matches_sample_requests -- --nocapture`

- Status: pass
- Artifact: `openspec/changes/refresh-tigerstyle-audit/evidence/aspen-client-api-request-metadata.txt`

### `cargo test -p aspen-ci-executor-nix detect_project_type -- --nocapture`

- Status: pass
- Artifact: `openspec/changes/refresh-tigerstyle-audit/evidence/aspen-ci-executor-nix-detect-project-type.txt`

### `cargo test -p aspen-ci-executor-nix find_missing_store_paths -- --nocapture`

- Status: pass
- Artifact: `openspec/changes/refresh-tigerstyle-audit/evidence/aspen-ci-executor-nix-find-missing-store-paths.txt`

### `cargo test -p aspen-forge-handler --all-features prepare_objects -- --nocapture`

- Status: pass
- Artifact: `openspec/changes/refresh-tigerstyle-audit/evidence/aspen-forge-handler-prepare-objects.txt`

### `cargo test -p aspen-forge-handler --all-features update_have_hashes -- --nocapture`

- Status: pass
- Artifact: `openspec/changes/refresh-tigerstyle-audit/evidence/aspen-forge-handler-update-have-hashes.txt`

### `cargo test -p aspen-core-essentials-handler alert_evaluate -- --nocapture`

- Status: pass
- Artifact: `openspec/changes/refresh-tigerstyle-audit/evidence/aspen-core-essentials-handler-alert-evaluate.txt`

### `cargo test -p aspen-cluster test_start_internal_rejects_duplicate_start -- --nocapture`

- Status: pass
- Artifact: `openspec/changes/refresh-tigerstyle-audit/evidence/aspen-cluster-start-internal.txt`

### `cargo test -p aspen-transport log_subscriber -- --nocapture`

- Status: pass
- Artifact: `openspec/changes/refresh-tigerstyle-audit/evidence/aspen-transport-log-subscriber.txt`

### `cargo test -p aspen-raft --features trust,secrets,testing try_collect_remote_share -- --nocapture`

- Status: pass
- Artifact: `openspec/changes/refresh-tigerstyle-audit/evidence/aspen-raft-secrets-at-rest.txt`

### `cargo test -p aspen-raft --features trust,secrets,testing ensure_share_quorum_reports_below_quorum -- --nocapture`

- Status: pass
- Artifact: `openspec/changes/refresh-tigerstyle-audit/evidence/aspen-raft-secrets-at-rest.txt`

### `cargo test -p aspen-raft --features trust,testing required_peer_confirmations -- --nocapture`

- Status: pass
- Artifact: `openspec/changes/refresh-tigerstyle-audit/evidence/aspen-raft-required-peer-confirmations.txt`

### `cargo test -p aspen-raft --features trust,testing share_collection_target -- --nocapture`

- Status: pass
- Artifact: `openspec/changes/refresh-tigerstyle-audit/evidence/aspen-raft-share-collection-target.txt`

### `cargo test -p aspen-forge --features git-bridge federation_export -- --nocapture`

- Status: pass
- Artifact: `openspec/changes/refresh-tigerstyle-audit/evidence/aspen-forge-federation-export.txt`

### `cargo check -p aspen-ci-executor-nix -p aspen-forge-handler --all-features -p aspen-client-api -p aspen-cluster -p aspen-transport -p aspen-core-essentials-handler -p aspen-raft -p aspen-jobs -p aspen-forge`

- Status: pass
- Artifact: `openspec/changes/refresh-tigerstyle-audit/evidence/cargo-check-targeted-packages.txt`

### `python3 scripts/tigerstyle-audit.py --json <targeted files>`

- Status: pass
- Artifact: `openspec/changes/refresh-tigerstyle-audit/evidence/tigerstyle-targeted-scan.json`

### `python3 scripts/tigerstyle-audit.py --json crates/aspen-ci-executor-nix/src/executor.rs crates/aspen-cluster/src/gossip/discovery/lifecycle.rs crates/aspen-transport/src/log_subscriber/connection.rs crates/aspen-cluster/src/bootstrap/node/storage_init.rs crates/aspen-raft/src/secrets_at_rest.rs`

- Status: pass
- Artifact: `openspec/changes/refresh-tigerstyle-audit/evidence/direct-hotspot-verification.json`

### `OPENSPEC_PREFLIGHT_ALLOW_UNTRACKED=1 scripts/openspec-preflight.sh refresh-tigerstyle-audit`

- Status: pass
- Artifact: `openspec/changes/refresh-tigerstyle-audit/evidence/openspec-preflight.txt`

## Notes

- `openspec/changes/refresh-tigerstyle-audit/evidence/tigerstyle-targeted-scan.txt` records that every named remediation target is absent from the scanner's `function_length`, `zero_assert_hotspot`, `usize_api`, and `verified_ambient_time` findings.
- `openspec/changes/refresh-tigerstyle-audit/evidence/direct-hotspot-verification.txt` gives direct per-function verification for `execute_build`, `start_internal`, `handle_log_subscriber_connection`, `create_raft_instance`, and `collect_shares`.
- `openspec/changes/refresh-tigerstyle-audit/evidence/cargo-check-targeted-packages.txt` is intentionally non-quiet so reviewers can inspect a non-empty successful transcript.
- Preflight used `OPENSPEC_PREFLIGHT_ALLOW_UNTRACKED=1` because the repo already contains unrelated untracked content at `openspec/changes/coordination-primitives-spec/`.
