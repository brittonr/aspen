# Retry observation domain binding

## Result and owner

The fixture now distinguishes equal ticks from different time domains during replay.
Molten fabric-time maintainers own this internal canonical boundary and its regression tests.
The executable fixture consumes the same private builder as the replay checks.
No core arithmetic, public API, port, profile, dependency, or effect authority changed.

This result does not complete F12 or Molten.
Strict Octet, nextest, complete feature/target coverage, full Nix, and lifecycle acceptance remain open.
The parent merge's configured Octet artifacts report `warning-only` with 6,787 warnings. Command exit 0 is not strict acceptance.
The inherited Tracey input correction has a separate worktree and evidence boundary.

## Source and counterexample

The parent is local merge `7c52757bbd9bfb44574a779eba25d58161c9e754`, with tree `b0cf08d651ef1e81be199b54aeb576143ddfe30c`.
A read-only reviewer identified a possible domain omission. Its report arrived before its five-minute deadline ended the process with exit 124.
That advisory report is not a lifecycle review receipt.

The new regression first ran against unchanged implementation code.
Both requests use the same profile, subject, generation, attempt, delay 128, and deadline tick 168.
One request uses virtual time. The other uses monotonic time.
The matching-domain control passed. Replay then accepted virtual observations for the monotonic request instead of returning typed divergence.
The red run returned exit 101, with one intended failure and 1,474 tests filtered out.
`retry-domain-red-source.patch` and the parent revision reproduce that source state.

## Correction and compatibility

`src/fabric_time/canonical/retry.rs` owns a private canonical retry builder.
It records the domain of each delay and deadline in the existing details field.
The fixture still obtains every valid plan from the pure core before any clock effect.
Replay recomputes complete canonical observations and compares their order, fields, and references.
It performs no timer effect and does not modify the supplied history.

The wrapped-history control retains a correct domain while changing delay and deadline values.
It cannot pass merely because domain data is absent.
A separate negative test rejects older domainless observations with otherwise correct ticks.
Changed identity, deadline, order, missing records, and empty history still reject.
Both cross-domain directions reject after the repair.

The profile and outer observation schema remain unchanged.
Only corrected arithmetic and domain-bound retry observations are admitted by this private fixture replay path.
Older observations are not silently rewritten or accepted under another arithmetic rule.
The existing ordinary retry observation remains at deadline 52.
The selected coordination-delivery profile remains fixed backoff without jitter.

Additional tests require exact timer observations, final tick 164, and a terminal event at tick 164.
The public saturation case still samples virtual tick 35 and delivers its timer at 163.
Request-profile mismatch and zero generation now join the exact-error, unchanged-state denial cases.

## Executed checks

| Check | Result |
|---|---|
| Cross-domain regression before the repair | Exit 101, intended replay acceptance failure |
| `cargo test --locked -p molten fabric_time` | Exit 0, 29 library tests and 2 CLI tests |
| `cargo clippy --locked --workspace --all-targets -- -D warnings` | Exit 0 |
| `cargo test --locked --workspace` | Exit 0, complete default-feature workspace |
| Public deterministic fixture | Exit 0, 37 emitted events across the bundle |
| Public report read-back | Exit 0, final tick 164, conformance true |
| Event supplied as a report | Exit 1 with the intended report-shape diagnostic |

The complete workspace run includes 1,476 main-library tests, 74 CLI tests, 370 core tests, and all eight native tests.
Compilation took 35.49s. The native suite took 231.10s.
No deadline, feature selection, build profile, native assertion, or caching rule changed.
These timings do not establish a performance gain or explain earlier duration differences.

The public report reference is `blake3:3fa3cce17944d1e95fd1fac660304bbfb51c8e9ec3f2dd61caf4f4569a8ff56a`.
The negative diagnostic is exactly:

```text
error: invalid harness artifact: expected canonical fabric-time run report
```

## Independent source review and lifecycle checks

A new bounded reviewer completed with exit 0 and reported no concrete finding in the two requested failure families.
It inspected semantic domain binding and observation ownership, then stopped at fifteen reads.
Its two review passes were local within that separate reader process, not two additional independent workers.
The coordinator separately checked the source hashes and executed tests.
The reviewer did not run commands, verify archives, inspect every adapter, or grant acceptance.

This invocation used `--no-extensions`, the same five-minute deadline, closed stdin, and the same explicit model identifier.
It completed in print mode. That observation does not establish why earlier reviewers timed out.
The report retains unmatched-model warnings for two unused account patterns.

`cairn validate --root . --strict` and the F12 tasks gate both returned exit 0.
The tasks receipt is `bafb242ab65216c2c42fe8509cc3918081dea364378088cfa387552d57b160a2`.
It contains empty acceptance IDs and review receipt hashes, advisory mode, and no declared probes.
Policy selection remains `legacy_default`, without an installation receipt.
These structural results and the advisory review do not complete the remaining three tasks.

## Reproduction and evidence

Commands run from the implementation worktree through `nix develop --no-write-lock-file`.
The environment uses `CARGO_BUILD_JOBS=2`, `RUST_TEST_THREADS=2`, and `CARGO_TARGET_DIR=/tmp/molten-completion-20260913-target`.
Each check keeps the existing eight-minute deadline.

```console
cargo test --locked -p molten --lib fabric_time::tests::retry::replay::replay_rejects_cross_domain_history_with_identical_ticks -- --exact
cargo test --locked -p molten fabric_time
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
cargo run --locked -p molten --bin molten -- fabric-time run-fixture --profile deterministic-simulation --out OUTPUT
cargo run --locked -p molten --bin molten -- fabric-time show OUTPUT/report.preserves
cargo run --locked -p molten --bin molten -- fabric-time show OUTPUT/evidence/0027-deadline.preserves
```

`inputs.b3` binds the selected source and build configuration.
`logs.tar.gz` retains lossless transcripts and direct exits, including the red source patch and manifest.
`public-artifacts.tar.gz` retains the emitted report, profiles, and event files.
`review-lifecycle.tar.gz` retains the review prompt, completed advisory report, lifecycle results, and direct exits.
`payloads.b3` binds these payloads.
The persistent campaign retains the original files outside disposable worktrees.

The first interrupted regression dispatch produced no task or log. The coordinator verified that absence before it queued the recorded run.
Replay comparison is not authentication, timer-effect verification, or authority for another timer.
Public `show` validates report shape, not arbitrary external event history.
The artifact bundle also contains unselected live observations. It is not a whole-bundle determinism or production-acceptance claim.
