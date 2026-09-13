# F12 fixture, adapter, and replay evidence

## Result and ownership

The corrected fixture passes its focused tests, the complete default-feature workspace tests, and workspace Clippy with `-D warnings`.
The public CLI emits and reads back its report.
The report finishes at tick 164, after the retry timer delivers at tick 163.
Strict Octet, required feature checks, nextest, and the complete Nix gate remain open.
This record does not complete F12 or authorize archive or release.

The immediate outcome is observable retry saturation through the existing fixture.
The durable capability is a private adapter harness and canonical retry comparison with positive and negative tests.
Molten fabric-time maintainers own this capability.
The existing `molten fabric-time run-fixture` command is its consumer and adoption path.
The commands, source identities, artifacts, and lossless logs below support repetition.

## Source and effect boundaries

The checked source builds on `fe8bec35a8762dcc0b13f7973a5435c2edcbf64b` in `implementation-retry`.
`inputs.b3` identifies the source and configuration inputs.
`payloads.b3` identifies the retained output bytes.
Historical core and delivery manifests retain their original revision scope.
They are not rewritten to claim the later documentation or fixture bytes.

`src/fabric_time/fixture/retry.rs` keeps its helpers inside `crate::fabric_time`.
The core still owns retry arithmetic, admission, timer scheduling, and timer transitions.
The shell samples the existing virtual clock and executes its admitted wait.
It publishes observations only after the timer transition succeeds.
No new public API, port, dependency, or runtime delivery policy was added.

The adapter tests cover valid saturation, invalid and missing jitter, exhaustion, stale generation, and true deadline overflow.
They compare the complete request, clock state, and existing event list after planner denial.
Clock profile or domain substitution causes no wait or publication.
A controlled timeout and an early clock return remain separate from planner denial.
The existing error conversion uses `MoltenError::InvalidHarness` for both domain and port failures, with distinct diagnostic text.
These tests preserve that conversion; they do not claim a new public error taxonomy.
The repeated delivery suite also covers fixed backoff and commit-before-effect ordering.

## Replay and cohort decision

Keep the current profile and observation schema.
The correction enforces the existing mathematical delay contract; it does not admit a second retry rule.
A new cohort is unnecessary while wrapped delays remain rejected rather than supported as another valid history.

The private replay helper recomputes the current plan from explicit inputs.
It compares complete canonical delay and deadline observations, including their order and evidence refs.
Valid fixed and saturated observations pass canonical byte read-back and replay.
Wrapped, changed, reordered, missing, and identity-mismatched observations produce `MoltenError::HarnessDivergence`.
The tests check that rejected history and input remain unchanged.
The executable fixture records `wrapped-replay-rejected` only after that rejection occurs.

This comparison does not authenticate history, verify timer effects, or authorize another timer.
It performs no clock operation.
The public `show` command checks report shape; it does not replay arbitrary external history.
No public history reader or migration path was added.
A future requirement to admit both arithmetic rules needs a separate cohort decision and validation.

## Public observations

The saturation case samples virtual tick 35, with base 2, attempt 63, cap 128, and an attempt limit of 65.
It records delay 128, deadline 163, and timer delivery at 163.
The run's shared clock reaches 164 before completion.
The ordinary jittered case retains deadline 52.
The selected coordination-delivery profile still uses fixed backoff without jitter.
The generic exponential case does not prove failure through that fixed profile or application-retry safety.

`artifacts/report.preserves` has report ref:

```text
blake3:b9d7e72621f99848e5328051bd5c6b7bbf0a9670ad5fa6580ad842abccbf6a17
```

The bundle retains all emitted events, including unselected live-profile observations.
Only the selected simulation report has the tested deterministic-report claim.
No production, VM, remote-clock, authentication, or performance claim follows from these artifacts.

## Executed checks

All exit files are direct command results. A filename containing `green` is not a verdict.

| Log stem | Result |
|---|---|
| `retry-fixture-pre-replay` | Existing fixture baseline, exit 0 |
| `retry-fixture-red` | Two intended missing-observation failures, exit 101 |
| `retry-fixture-green` | First implementation: 26 library and two CLI tests pass, exit 0 |
| `retry-fixture-clippy` | First implementation: workspace Clippy passes, exit 0 |
| `retry-fixture-public` | First implementation emits artifacts, exit 0; clock ownership is not acceptable |
| `retry-fixture-clock-red` | Completion precedes retry delivery: one intended failure, exit 101 |
| `retry-fixture-clock-green` | Shared-clock repair: 27 library and two CLI tests pass, exit 0 |
| `workspace-tests-retry-fixture` | Complete workspace command passes, exit 0; includes 1,474 library, 370 core, and eight native tests |
| `retry-fixture-clippy-shared-clock` | Workspace, all-target Clippy with `-D warnings` passes, exit 0 |
| `retry-fixture-public-shared-clock` | Corrected public fixture emits artifacts, exit 0 |
| `retry-fixture-readback` | Corrected report read-back passes, exit 0 |
| `retry-fixture-readback-negative` | Event supplied as a report is rejected, exit 1 |
| `retry-fixture-core-repeat` | Focused core repeat: 370 pass, exit 0 |
| `retry-fixture-delivery-repeat` | Focused delivery repeat: 21 pass, exit 0 |
| `retry-fixture-review` | Five-minute reviewer deadline, exit 124; no report |
| `cairn-retry-fixture-validation` | Structural validation passes for 55 changes, exit 0; no reported issues or findings |
| `cairn-retry-fixture-tasks` | Tasks gate passes, exit 0; acceptance IDs and review receipt hashes remain empty |

Cairn selects the unchanged generated policy through `legacy_default`, without an installation receipt.
These structural checks do not establish implementation truth, review approval, or release acceptance.

The negative CLI diagnostic is exact:

```text
error: invalid harness artifact: expected canonical fabric-time run report
```

The passing workspace command built in 28.92 seconds.
Its main library tests took 39.71 seconds; its native suite took 226.23 seconds.
These timings do not establish a cause for earlier timeouts or a measured improvement.
No deadline, assertion, feature selection, build profile, or native caching rule was relaxed.
The workspace command used its normal default features, not the required complete feature matrix.

## Reproduction

Run from the repository root in the selected source revision.
The campaign used these bounds and environment settings:

```console
export CARGO_BUILD_JOBS=2
export CARGO_TARGET_DIR=/tmp/molten-completion-20260913-target
export RUST_TEST_THREADS=2
CHECK_DEADLINE=8m
timeout "$CHECK_DEADLINE" nix develop --no-write-lock-file -c cargo test --locked -p molten fabric_time
timeout "$CHECK_DEADLINE" nix develop --no-write-lock-file -c cargo test --locked -p molten-core --lib
timeout "$CHECK_DEADLINE" nix develop --no-write-lock-file -c cargo test --locked -p molten --lib coordination_delivery::tests
timeout "$CHECK_DEADLINE" nix develop --no-write-lock-file -c cargo test --locked --workspace
timeout "$CHECK_DEADLINE" nix develop --no-write-lock-file -c cargo clippy --locked --workspace --all-targets -- -D warnings
```

For the public path, select a fresh task-owned output directory:

```console
OUTPUT=artifacts/retry-fixture-check
test ! -e "$OUTPUT" && timeout "$CHECK_DEADLINE" nix develop --no-write-lock-file -c cargo run --locked -p molten --bin molten -- fabric-time run-fixture --profile deterministic-simulation --out "$OUTPUT"
timeout "$CHECK_DEADLINE" nix develop --no-write-lock-file -c cargo run --locked -p molten --bin molten -- fabric-time show "$OUTPUT/report.preserves"
timeout "$CHECK_DEADLINE" nix develop --no-write-lock-file -c cargo run --locked -p molten --bin molten -- fabric-time show "$OUTPUT/evidence/0027-deadline.preserves"
```

The final command must exit 1 with the diagnostic above.
Do not treat another nonzero cause as a passing negative check.

## Rejected attempt and review limit

The first implementation created a separate virtual clock with the same profile.
Its report ended at tick 36 while its retry timer delivered at tick 168.
The new completion regression exposed that mismatch despite earlier unit and Clippy passes.
The repair uses the run's shared clock and samples its current tick.
The first source snapshot predates the added completion regression.
Its source and public artifacts remain in the two `rejected-separate-clock` archives.

The independent reviewer returned no report before its deadline.
The coordinator supplied source review and the executed clock counterexample, not an independent review receipt.
Required lifecycle review and acceptance gates remain separate.
