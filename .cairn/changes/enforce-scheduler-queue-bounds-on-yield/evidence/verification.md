# Verification: bounded Yield admission (F10)

## Baseline

The pre-change checkout at `831df7325` (branch `molten`) let `Yield` move a running
occurrence into ready without queue admission: with queue bound one, A running, and B ready,
the transition produced two ready entries.

The pre-change reproduction ran this change's core tests against the pre-change source in a
clean checkout: 6 of the scheduler tests failed, including
`yield_denies_at_the_ready_bound_and_preserves_state`,
`backpressure_profile_maps_denied_yield_to_backpressure`, and
`corrected_yield_admission_diverges_from_an_over_capacity_history`; the positive controls
(`yield_admits_with_ready_capacity_and_takes_a_fresh_position`,
`yield_denies_invalid_phase_and_exhausted_sequence_without_mutation`) passed.

## Implementation evidence

- `crates/molten-core/src/fabric_time/scheduler/mod.rs`: `yield_occurrence` checks the
  running phase, applies `ready_overload_action`, and only then calls `transition_phase`, so
  a denied yield publishes the admitted overload action with byte-identical state and no
  fresh enqueue position. Successful yield keeps the checked fresh position and the
  `Yielded` action.
- `ready_overload_action` is the single ready-capacity contract for new wake, blocked wake,
  and yield. Only a new occurrence charges an active slot; existing occurrences charge ready
  capacity alone.
- `src/fabric_time/shell.rs`: extension commands translate the core decision directly, so a
  denied yield produces `Reject` or `Backpressure` and the running occurrence keeps its
  ownership; no accepted-yield effect is emitted.
- `docs/fabric-time-scheduler-runtime.md`: records the shared admission rule, the overload
  caller behavior, and the replay-divergence decision for histories that relied on the
  former over-capacity yield.

## Checks

Executed in `/home/brittonr/git/OnixResearch/aspen-f12`, after
`cargo clean -p molten-core -p molten` (sibling worktrees share one cargo target directory,
and a stale artifact masked fixes in earlier runs):

- `cargo fmt --all` then `cargo fmt --check`: clean.
- `cargo check --workspace --all-targets`: no errors.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo test -p molten-core --lib`: 388 passed (pre-change 377).
- `cargo test -p molten --lib`: 1494 passed (pre-change 1491).
- focused: `cargo test -p molten-core --lib -- fabric_time` 41 passed;
  `cargo test -p molten --lib -- fabric_time` 26 passed.
- `cargo octet check --artifact-dir target/octet-f09f10-2`: 3615 findings, 0 errors,
  `warning-only`, equal to the pre-change baseline.
- Cairn: `validate --root . --strict` valid; `gate proposal|design|tasks` valid.

Not run: the repository strict Octet gate (designed to deny `warning-only` artifacts) and
Nix checks.

## Traceability

- `r[molten.audit_f10.queue]`: `yield_denies_at_the_ready_bound_and_preserves_state`,
  `yield_admits_with_ready_capacity_and_takes_a_fresh_position`,
  `backpressure_profile_maps_denied_yield_to_backpressure`.
- `r[molten.audit_f10.shared_admission]`:
  `new_wake_denies_at_the_active_bound_while_ready_capacity_exists` plus the blocked-resume
  control in the F09 package.
- `r[molten.audit_f10.atomicity]`:
  `yield_denies_invalid_phase_and_exhausted_sequence_without_mutation`,
  `stale_generation_wake_and_yield_discard_without_effects`,
  `extension_context_reports_overload_for_yield_at_the_ready_bound`.
- `r[molten.audit_f10.validation]`:
  `corrected_yield_admission_diverges_from_an_over_capacity_history` plus the focused
  suites and the pre-change reproduction.

## Caveats

- No new live-versus-simulation adapter pair test was added; adapter action translation
  remains covered by the extension-shell tests above. Task 7 stays unchecked for that
  reason.
- Tasks 10 and 11 remain unchecked: the strict Octet gate and the Nix checks were not run.
- Nix checks were not run; no dependency, lockfile, or flake input changed.

## Claim boundary

The evidence shows bounded yield admission, overload translation, and state preservation for
the pure scheduler and the extension shell over supplied profiles and commands. It does not
prove global liveness, measured performance, live-adapters equivalence, or release
readiness.
