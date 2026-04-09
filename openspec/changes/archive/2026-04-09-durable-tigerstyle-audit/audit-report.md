# Tiger Style Audit Report

Status: remediation complete

This report is the durable record for the Tiger Style audit that started this change. It keeps the current hotspot inventory, the scanner contract, and the evidence paths in one repo-visible place.

## Scope

- Scan target: `crates/*/src/**/*.rs`
- Scanner command: `./scripts/tigerstyle-audit.py crates`
- Hotspot rules:
  - function bodies over 70 lines
  - overlong functions with zero `assert!` / `debug_assert!` calls
  - recursive function bodies
  - `SystemTime::now()` or `Instant::now()` under `src/verified/`
  - public or verified signatures that expose `usize`
- Manual review focus: production handlers and pure APIs called out by the initial hotspot list

## Methodology

1. Run `./scripts/tigerstyle-audit.py --pretty --json crates` to emit a machine-readable inventory.
2. Review the saved transcript at `openspec/changes/durable-tigerstyle-audit/evidence/initial-scan.txt`.
3. Spot-check the highest-ranked production hotspots before turning them into refactor tasks.
4. Keep the trait-declaration regression fixture green with `python3 tools/tigerstyle/test_tigerstyle_audit.py`.

## Initial scan summary

Source: `openspec/changes/durable-tigerstyle-audit/evidence/initial-scan.json`

| Measure | Count |
| --- | ---: |
| Files scanned | 1285 |
| Functions scanned | 16911 |
| `function_length` hotspots | 363 |
| `zero_assert_hotspot` hotspots | 363 |
| `recursion` hotspots | 842 |
| `usize_api` hotspots | 176 |
| `verified_ambient_time` hotspots | 1 |
| Parse errors | 5 |

## Ranked hotspot table

The scanner ranks by rule first, then by line count. This table calls out the highest-value production hotspots from the initial scan.

| Rank | File | Function | Rules | Lines |
| ---: | --- | --- | --- | ---: |
| 1 | `crates/aspen-forge-handler/src/executor.rs` | `execute` | `zero_assert_hotspot`, `function_length` | 692 |
| 2 | `crates/aspen-core-essentials-handler/src/coordination.rs` | `handle` | `zero_assert_hotspot`, `function_length` | 546 |
| 3 | `crates/aspen-ci-handler/src/handler/pipeline.rs` | `handle_trigger_pipeline` | `zero_assert_hotspot`, `function_length` | 287 |
| 4 | `crates/aspen-auth/src/capability.rs` | `authorizes` | `zero_assert_hotspot`, `function_length` | 184 |
| 5 | `crates/aspen-auth/src/capability.rs` | `contains` | `zero_assert_hotspot`, `function_length` | 164 |
| 6 | `crates/aspen-forge/src/git/merge.rs` | `merge_trees_recursive` | `zero_assert_hotspot`, `function_length`, `recursion` | 155 |
| 7 | `crates/aspen-ci-core/src/verified/pipeline.rs` | verified/public `usize` API set | `usize_api` | n/a |
| 8 | `crates/aspen-forge/src/verified/ref_validation.rs` | verified/public `usize` API set | `usize_api` | n/a |
| 9 | `crates/aspen-cluster/src/gossip/discovery/helpers.rs` | verified/public `usize` API set | `usize_api` | n/a |

## Known scanner limitations

- The parser is brace- and semicolon-aware, but it is still a lightweight scanner. It does not expand macros or fully parse every Rust edge case.
- The current run still reports five parse errors in complex files:
  - `crates/aspen-ci/src/config/loader.rs`
  - `crates/aspen-forge/src/cob/review.rs`
  - `crates/aspen-forge/src/git/bridge/tests.rs`
  - `crates/aspen-nostr-relay/src/connection.rs`
  - `crates/aspen-rpc-handlers/src/registry.rs`
- The recursion rule is purely lexical. It is good for prioritization, not proof.
- The `usize_api` rule works at the signature level. It does not yet classify whether a `usize` is an internal adapter boundary or part of a durable public contract.

## Regression coverage for the declaration-vs-body bug

- Scanner: `scripts/tigerstyle-audit.py`
- Fixture: `tools/tigerstyle/fixtures/trait_declaration_false_positive.rs`
- Regression test: `tools/tigerstyle/test_tigerstyle_audit.py`
- Saved run: `openspec/changes/durable-tigerstyle-audit/evidence/scanner-regression.txt`

The fixture pins the failure mode from the exploratory scan: a trait declaration that ends in `;` must not be counted as a function body just because an impl body for the same name appears later in the file.

## First remediation slice update

Source: `openspec/changes/durable-tigerstyle-audit/evidence/post-slice-scan.json`

The first production slice is now in the tree:

- `crates/aspen-ci-handler/src/handler/pipeline.rs:handle_trigger_pipeline` was split into smaller trigger helpers and no longer appears in the overlong-function inventory.
- `crates/aspen-auth/src/capability.rs:Capability::authorizes` and `Capability::contains` were split into family-specific helpers and no longer appear in the overlong-function inventory.
- The initial `usize` cleanup set no longer shows `usize_api` hits for:
  - `crates/aspen-ci-core/src/verified/pipeline.rs`
  - `crates/aspen-forge/src/verified/ref_validation.rs`
  - `crates/aspen-cluster/src/gossip/discovery/helpers.rs`

### Before / after counts for the first slice

| Rule | Initial scan | Post-slice scan | Delta |
| --- | ---: | ---: | ---: |
| `function_length` | 363 | 360 | -3 |
| `zero_assert_hotspot` | 363 | 360 | -3 |
| `usize_api` | 176 | 171 | -5 |
| `recursion` | 842 | 842 | 0 |
| `verified_ambient_time` | 1 | 1 | 0 |

The scan still reports `crates/aspen-forge/src/verified/ref_validation.rs:validate_ref_name` as overlong after the `usize` cleanup. That length issue stays in the backlog for a later slice; the current task here was only the `usize` leak.

## Phase 4 dispatcher update

Source: `openspec/changes/durable-tigerstyle-audit/evidence/post-phase4-scan.json`

The second remediation slice removed the original top-level dispatcher hotspots from the scan:

- `crates/aspen-core-essentials-handler/src/coordination.rs:handle`
- `crates/aspen-forge-handler/src/executor.rs:execute`

That work split the dispatchers by request family and added direct execution coverage for representative coordination and forge requests, including unsupported-request error behavior. The post-phase-4 scan still shows follow-on hotspots in some newly extracted helpers (`renew_lock`, `execute_federation_request`, `execute_git_bridge_request`, `execute_nostr_request`, `execute_repo_request`, and `request_group`). Those are smaller and more localized than the original monoliths, but they remain backlog candidates for later cleanup.

## Current status

This change is complete for its scoped tasks. The audit artifacts are durable, the scanner regression fixture is committed, both remediation slices landed with targeted coverage, and the saved preflight transcript shows `19 / 19` tasks checked. Remaining scan hotspots outside this task list stay as backlog for later changes rather than blocking this one.
