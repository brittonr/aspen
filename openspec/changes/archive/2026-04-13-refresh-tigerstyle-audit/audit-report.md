# Tiger Style Audit Report

Status: remediation slice complete

## Scope

- Backlog-input scan target: `crates/*/src/**/*.rs`
- Scanner command: `python3 scripts/tigerstyle-audit.py --json crates`
- Backlog-input evidence:
  - `openspec/changes/refresh-tigerstyle-audit/evidence/current-scan.json`
  - `openspec/changes/refresh-tigerstyle-audit/evidence/current-scan.txt`
  - `openspec/changes/refresh-tigerstyle-audit/evidence/compare-to-2026-04-09.txt`
- Post-remediation verification evidence:
  - `openspec/changes/refresh-tigerstyle-audit/evidence/tigerstyle-targeted-scan.json`
  - `openspec/changes/refresh-tigerstyle-audit/evidence/tigerstyle-targeted-scan.txt`
  - `openspec/changes/refresh-tigerstyle-audit/evidence/implementation-diff.txt`
- Baseline for comparison:
  - `openspec/changes/archive/2026-04-09-durable-tigerstyle-audit/evidence/post-phase4-scan.json`

## Methodology

1. Refresh the Tiger Style audit against the 2026-04-09 baseline and save repo-visible scan output.
2. Rank the highest-value production hotspots from that scan.
3. Implement the first remediation slice against those named hotspots.
4. Re-run targeted scanner checks and targeted crate verification for the remediated files.

## Backlog-input scan summary

Source: `openspec/changes/refresh-tigerstyle-audit/evidence/current-scan.json`

| Measure | Count |
| --- | ---: |
| Files scanned | 1297 |
| Functions scanned | 17364 |
| `function_length` hotspots | 375 |
| `zero_assert_hotspot` hotspots | 375 |
| `recursion` hotspots | 865 |
| `usize_api` hotspots | 172 |
| `verified_ambient_time` hotspots | 1 |
| Parse errors | 6 |

## Delta from 2026-04-09 baseline

Source: `openspec/changes/refresh-tigerstyle-audit/evidence/compare-to-2026-04-09.txt`

| Measure | 2026-04-09 | 2026-04-13 | Delta |
| --- | ---: | ---: | ---: |
| Files scanned | 1285 | 1297 | +12 |
| Functions scanned | 16990 | 17364 | +374 |
| `function_length` hotspots | 364 | 375 | +11 |
| `zero_assert_hotspot` hotspots | 364 | 375 | +11 |
| `recursion` hotspots | 842 | 865 | +23 |
| `usize_api` hotspots | 171 | 172 | +1 |
| `verified_ambient_time` hotspots | 1 | 1 | 0 |
| Parse errors | 5 | 6 | +1 |

## Completed in this change

This remediation slice removed the named backlog targets by:

- splitting CI executor / flake-lock monoliths into bounded helpers
- splitting federation handlers into smaller phases
- replacing duplicated request metadata mega-matches with one table-driven source
- splitting discovery, log-subscriber, alert, trust, secrets, and storage orchestration helpers
- removing the highest-priority production `usize_api` leaks named by the backlog
- fixing Tiger Style scanner regressions for lifetimes and doc-comment ambient-time false positives

Changed production files include:

- `crates/aspen-ci-executor-nix/src/{executor.rs,flake_lock.rs}`
- `crates/aspen-client-api/src/{lib.rs,messages/mod.rs,messages/request_metadata.rs}`
- `crates/aspen-cluster/src/{bootstrap/node/storage_init.rs,gossip/discovery/lifecycle.rs,gossip/discovery/mod.rs}`
- `crates/aspen-core-essentials-handler/src/core.rs`
- `crates/aspen-forge-handler/src/handler/handlers/{federation.rs,federation_git.rs}`
- `crates/aspen-forge/src/{git/bridge/exporter.rs,resolver.rs}`
- `crates/aspen-jobs/src/{distributed_pool.rs,scheduler.rs}`
- `crates/aspen-raft/src/{node/trust.rs,secrets_at_rest.rs}`
- `crates/aspen-transport/src/log_subscriber/connection.rs`
- `scripts/tigerstyle-audit.py`
- `tools/tigerstyle/test_tigerstyle_audit.py`

## Post-remediation targeted verification

Source: `openspec/changes/refresh-tigerstyle-audit/evidence/tigerstyle-targeted-scan.txt`

The targeted post-remediation scan shows these named backlog functions no longer trigger `function_length`, `zero_assert_hotspot`, `usize_api`, or `verified_ambient_time` findings:

- `deserialize`
- `try_native_build`
- `execute_build`
- `federation_import_objects`
- `handle_federation_fetch_refs`
- `handle_federation_bidi_sync`
- `sync_from_origin`
- `variant_name`
- `required_app`
- `start_internal`
- `handle_log_subscriber_connection`
- `handle_alert_evaluate`
- `collect_old_shares_for_reconfiguration`
- `probe_for_peer_expungement`
- `rotate_trust_after_membership_change`
- `collect_shares`
- `create_raft_instance`
- `recover_schedules`
- `export_commit_dag_blake3`
- `create_worker_group`
- `get_if_initialized`

The targeted scan also reports `parse_errors: []` for the remediated scanner cases.

## Remaining backlog after this slice

The repo still has broader Tiger Style debt outside the named slice, including other large handlers, recursion-name noise, and lower-priority `usize_api` sites. This change does not claim whole-repo remediation complete. It claims that the named first-slice targets were refactored and re-verified.
