# Verification: saturate exponential retry delay

## Baseline

The pre-change checkout at `0c5ba118d` (branch `molten`, the integrated drain state) still
wrapped the exponential delay: `2_u64.checked_shl(63)` is `0`, so the plan capped a wrapped
`0` instead of saturating. The audit records this as F12.

The pre-change reproduction ran this change's core tests against the pre-change source in a
clean checkout. Four tests failed, including
`retry_exponential_matches_a_wide_integer_reference`
(`base 2, attempt 63: left: 0, right: 128`), which is the audit's exact trigger.

## Implementation evidence

- `crates/molten-core/src/fabric_time/lease.rs`: exponential backoff now goes through
  `capped_exponential_delay`, a total function with a width guard
  (`attempt >= u64::BITS` saturates) and a checked multiply whose failure saturates at the
  policy maximum. There is no panic path, no attempt-sized loop, and no narrowing error.
- `crates/molten-core/src/fabric_time/tests.rs`: saturation and width boundaries, a
  wide-integer reference comparison across the admissible base range, jitter capping after
  saturation, and jitter-addition overflow rejection before the cap.
- `src/fabric_time/fixture.rs` and `src/fabric_time/history/mod.rs`: the retry fixture emits
  canonical delay and deadline events, and `replay_fixture_retry` recomputes them from
  explicit inputs and rejects divergent recorded history instead of rewriting it.
- `src/fabric_time/tests.rs` and `src/fabric_time/tests/replay.rs`: replay controls
  (matching saturated and fixed observations accepted; wrapped, malformed, and individually
  changed events rejected).
- `docs/fabric-time-scheduler-runtime.md`: records the width guard, the checked
  multiplication, and the fixed-delay delivery-profile non-claim.

## Checks

Executed in `/home/brittonr/git/OnixResearch/aspen-f12`, after
`cargo clean -p molten-core -p molten` (sibling worktrees share one cargo target
directory, and a stale `molten-core` artifact masked the fix in one earlier run):

- `cargo fmt --all` then `cargo fmt --check`: clean.
- `cargo check --workspace --all-targets`: no errors.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo test -p molten-core --lib`: 377 passed (pre-change 372).
- `cargo test -p molten --lib`: 1491 passed (pre-change 1484).
- focused: `cargo test -p molten-core --lib -- fabric_time` 30 passed;
  `cargo test -p molten --lib -- fabric_time` 23 passed.
- `cargo octet check --artifact-dir target/octet-f12-3`: 3615 findings, 0 errors,
  `warning-only`, equal to the integrated pre-change baseline, so this change adds no
  finding. Two intermediate revisions added `module_file_count` and
  `path_segment_repetition` findings; the final layout keeps `src/fabric_time/` at seven
  source files with the replay boundary in `history/mod.rs`.
- Cairn: `validate --root . --strict` valid; `gate proposal|design|tasks` valid for this
  change.

## Traceability

- `r[molten.audit_f12.saturation]` and `r[molten.audit_f12.bounds]`:
  `capped_exponential_delay` with
  `retry_exponential_delay_saturates_at_arithmetic_boundaries` and
  `retry_exponential_matches_a_wide_integer_reference`.
- `r[molten.audit_f12.compatibility]`: fixed-delay and jitter admission controls
  (`retry_fixed_delay_and_rejections_remain_exact`,
  `retry_caps_valid_jitter_after_saturation`,
  `retry_rejects_jitter_addition_overflow_before_the_cap`).
- `r[molten.audit_f12.validation]`: `src/fabric_time/tests/replay.rs` controls plus the
  pre-change reproduction recorded above.

## Caveats

- This slice deliberately excludes the older `drain/molten-retry-saturation-20260908`
  branch's Nix surface (`flake.nix` input, `flake.lock`, `crate-hashes.json`,
  `build-plan.json`, `release-policy-build-plan.json`) and its `cairn-policy/consumer.ncl`
  and `tools/tracey` changes. Those remain unintegrated, and that branch's own evidence
  records a strict-Octet `integration-failure` and an interrupted Nix run.
- The repository strict Octet gate was not run: it denies `warning-only` artifacts by
  design while the workspace carries inherited warning debt, so a denial is not a pass.
- Nix checks were not run.
- The change's tasks 10 and 11 remain unchecked because they require the strict Octet gate
  and the Nix checks named above.

## Claim boundary

The evidence shows the admitted retry planner saturates delay decisions and that the
fixture replay rejects divergent recorded observations for this source and policy cohort. It
does not prove live timer behavior, transport delivery, or release readiness.
