# Verification: Octet burn-down, ambient clock and structural-scan recursion

Base: `octet-burndown-safety-collections-runtime` (`247b00e52`). Octet: pinned `octet-toolchain` `fc38f593`. Private
`CARGO_TARGET_DIR`; `nice -n 10`, `CARGO_BUILD_JOBS=16`, Nix `--max-jobs 2`.

## Octet

| Scope | Base findings | After | `ambient_clock` findings (sites) | `no_recursion` findings (sites) |
|---|---:|---:|---:|---:|
| workspace (`cargo octet check`) | 3267 | 3248 | 17 (9) → 0 | 2 (1) → 0 |
| `-p molten --lib` | 1388 | 1379 | 8 (8) → 0 | 1 (1) → 0 |

`octet-lint-diff.txt` shows that no other family changed.

## Allows added (exactly two)

- `LiveClockAdapter::new` — `tigerstyle::ambient_clock`: "LiveClockAdapter is the documented live monotonic clock
  capability; it anchors its origin at the host monotonic clock once, and every other reader goes through the
  TimerClockAdapter port".
- `LiveClockAdapter::observe_wall` — `tigerstyle::ambient_clock`: "LiveClockAdapter is the documented live wall-clock
  capability; observe_wall is the single place that reads the host wall clock and returns it as a canonical
  uncertainty-bounded observation".

`git diff | grep '^+.*allow('` lists exactly these two. The third adapter site, `await_ticks`, and the six supervision
sites are repaired through `TickDeadline` and `SupervisionDeadline`.

## Bounds and tests

- `tick_deadline_expires_exactly_at_its_timeout_on_the_virtual_clock`: one tick before the deadline, 1 tick remains
  and the deadline is not expired. At the deadline it is expired.
- `tick_deadline_denies_a_timeout_past_the_clock_domain`: the largest representable timeout is admitted, and one more
  tick is denied.
- `supervision_deadline_admits_the_supervision_bound_and_denies_one_past`: 3600 s is admitted (the Sightglass maximum),
  3600 s + 1 ns is denied, and a zero timeout is already expired.
- Structural scan: `structural_scan_reports_the_first_preorder_match_path`,
  `structural_scan_admits_the_exact_depth_and_node_bounds_and_denies_one_past`,
  `structural_scan_bounds_wide_containers_by_the_node_budget`, and the existing scan tests. A throwaway differential test
  compared the iterative scan with the previous recursive one over 1,680,000 scans (generated values × 7 limit pairs × 4
  predicates × 3 scopes). Results and error messages were identical. The test source is in
  `structural-scan-differential.txt`, and the test was removed after it passed.

## Rust gates

- `cargo fmt --check`: exit 0. `cargo clippy --workspace --all-targets -- -D warnings`: exit 0.
- Focused: `cargo test -p molten --lib -- fabric_time structural_scan cluster_harness wasm::performance
  fabric_consistency::raft::live_process`: 44 passed, 0 failed.
- `cargo test --workspace --no-fail-fast`: exit 0, 2083 passed, 0 failed, 0 ignored. The live cluster-harness, Raft
  live-process, and distinct-process transport tests exercise `SupervisionDeadline` on real child processes.

## Receipt identity

- `fixture-receipt-comparison.txt`: all 6 `examples/*.preserves` harness suites give identical exit codes and report
  and gate-receipt BLAKE3 hashes on the base binary and this tree.
- `simulation-fixture-comparison.txt`: `molten fabric-simulation run` (100 artifacts) and `shrink` (3) are
  byte-identical to the base.

## Flake checks

`nix build .#checks.x86_64-linux.<check>` passed: `fabric-port-boundaries`, `wasm-component-performance-profile`,
`native-system-extension-host-profile`, `cap-std-test-workspaces`, and `fabric-execution-octet-deny-all`.

## Review and lifecycle

The no-spec plan review approves the proposal, design, acceptance, and tasks with no findings. The proposal, design,
and tasks gates pass, and `cairn validate --strict` reports no issues.
