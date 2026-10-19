# Reviewed benchmark phase group preflight

## Count owner and boundary placement

The supported profile validates `profile.phases` and each suite's
`suite.phases` against `PerformancePhase::ALL`, whose existing
`PERFORMANCE_PHASE_COUNT` is three. `BenchmarkRunInput` carries a
validated suite, so after the existing profile, suite and run-input
checks, reject `input.phases.len() > input.suite.phases.len()` before
passing ownership to `normalize_phase_groups`. This retains
invalid profile, suite, host, workload and recorded-effect precedence
over the new oversized phase denial.

`BenchmarkRun` carries only `suite_ref`, not the suite. Its public
validator therefore rejects `run.phases.len() > PERFORMANCE_PHASE_COUNT`
at function entry, before formatting malformed ref diagnostics,
scanning forged phase/sample groups or recomputing `benchmark_run_ref`.
Both entry points use the same static, single-blocker denial;
normalizing a valid input still sorts its phase groups and samples
exactly as before. No phase-group vectors, duplicate values or
existing admitted-invalid diagnostics are silently collapsed.

Keep the limit as an upper bound rather than an exact-equality
shortcut: empty, missing, duplicate, unsorted and malformed groups
at zero through three retain existing validation and precedence.
The new guard rejects a fourth supplied group before per-group
allocation and formatting; it does not claim a sample-count repair.

## Verification

Red-to-green behavior tests on both entry points exercise exactly
three unordered phases, admitted malformed and duplicate groups,
invalid workload priority, and four caller-built/forged groups.
Compile an external public facade consumer because no production CLI
constructs runs internally. Compose source/tests/docs/Cairn without
overwriting existing changes, then check focused fmt, Clippy,
nonincremental library Octet and strict Cairn in both trees;
record any residual static warning separately from runtime proof.
