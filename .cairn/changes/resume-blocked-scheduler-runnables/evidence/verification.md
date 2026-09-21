# Verification: blocked runnable resume (F09)

## Baseline

The pre-change checkout at `831df7325` (branch `molten`) denied `Wake -> Block -> Wake` for
one current-generation occurrence with `DuplicateRunnable`, and the system-extension shell
charged a resumed occurrence as new work against the active envelope.

The pre-change reproduction ran this change's core tests against the pre-change source in a
clean checkout: 6 of the scheduler tests failed, including
`wake_resumes_a_blocked_occurrence_at_the_active_limit`,
`wake_resume_enters_fifo_order_behind_older_ready_work`,
`wake_resume_denies_when_the_ready_queue_is_full`, and
`wake_resume_overflow_preserves_state`; the pre-existing controls
(`new_wake_denies_at_the_active_bound_while_ready_capacity_exists`,
`yield_admits_with_ready_capacity_and_takes_a_fresh_position`,
`yield_denies_invalid_phase_and_exhausted_sequence_without_mutation`) passed.

## Implementation evidence

- `crates/molten-core/src/fabric_time/scheduler/mod.rs`: `wake` now classifies an existing
  occurrence by phase. A `blocked` occurrence resumes in place: it takes a checked fresh
  enqueue position, the supplied priority, and a reset wait counter, and it charges only
  ready capacity because it already holds its active slot. `ready`, `running`, `completed`,
  and `cancelled` occurrences stay duplicates. New work still charges both the active
  envelope and the ready queue, and a denied decision returns `Reject` or `Backpressure`
  with byte-identical state.
- Shared admission lives in `ready_overload_action` and is used by new wake, blocked wake,
  and yield (the F10 package's requirement).
- `src/fabric_time/shell.rs`: the system-extension Wake precheck no longer charges a blocked
  occurrence against `max_runnables`; new occurrences keep the extension envelope.
- `docs/fabric-time-scheduler-runtime.md`: records resume, duplicate, ordering, staleness,
  and terminal-retention ownership.

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

Not run: the repository strict Octet gate (it denies `warning-only` artifacts by design
while the workspace carries inherited warning debt) and Nix checks.

## Traceability

- `r[molten.audit_f09.resume]`: `wake_resumes_a_blocked_occurrence_at_the_active_limit`,
  `wake_denies_non_blocked_duplicates_without_mutation`,
  `extension_context_resumes_a_blocked_occurrence_at_the_active_limit`.
- `r[molten.audit_f09.queue]`: `wake_resume_denies_when_the_ready_queue_is_full`.
- `r[molten.audit_f09.ordering]`: `wake_resume_enters_fifo_order_behind_older_ready_work`,
  `wake_resume_overflow_preserves_state`.
- `r[molten.audit_f09.validation]`: `stale_generation_wake_and_yield_discard_without_effects`,
  the focused suites above, and the pre-change reproduction.

## Caveats

- No new live-versus-simulation adapter pair test was added; adapter routing remains covered
  by the existing `live_scheduler_wake_shell_routes_only_admitted_wake_transitions` and the
  shell resume test above. Task 7 remains unchecked for that reason.
- Tasks 10 and 11 remain unchecked: the strict Octet gate and the Nix checks were not run.
- Nix checks were not run; no dependency, lockfile, or flake input changed.
- F11 terminal retention is unchanged by this package; only the documentation records the
  ownership boundary.

## Claim boundary

The evidence shows bounded resume, ordering, and rejection semantics for the pure scheduler
and the extension shell over supplied profiles and commands. It does not prove global
liveness, measured performance, or release readiness.
