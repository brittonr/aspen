# F11 execution and compatibility evidence — 2026-10-21

## Scope and decisions

Change: `bound-terminal-scheduler-retention`, on isolated branch `cairn/next-audit-20261021`, based on `f8ccefe6031b6746d2c8661f4599d1cdcf9bfa74`.

The archived pre-edit session reproduced the accumulation finding: with active limit one, completing a second occurrence retained two terminal records. That observation establishes accumulation, not the correctness of the new fences. The normal regression `terminal_scheduler_records_stay_within_derived_retention_bound` now exercises sixteen fresh occurrences with the same logical name.

The implementation derives terminal retention from the active limit, reserves checked active-plus-terminal physical slots, reclaims the oldest terminal positions without moving active work, and binds fresh issuance to a nonzero monotonic occurrence sequence and a generation high-water fence. Rejected commands do not advance that fence, reclaim history, change accounting, or deliver callbacks. Cleanup checks its entire terminal-sequence increment before mutation.

The scheduler port/callback/event and capacity-plan identity move to the occurrence-fenced `v2` cohort. Base time profiles, clock/timer/entropy ports and non-scheduler event wrappers remain `v1`. Legacy/missing/mistyped/zero-fenced callback decoding denies; old receipts are not upgraded into current work. Vacant-slot history is unavailable detail, not evidence of an invented terminal outcome.

The retained pre-edit scheduler-filter output records 13 root tests and nine core tests passing, including the owning wake shell, blocked resume, Ready saturation, replay rejection, capacity fencing, and invalid-plan controls. This baseline did not contain the new F11 occurrence/eviction regressions. The accumulation reproduction and the post-change fence checks are separate observations.

## Executed focused checks

Inside the repository Nix shell, with an owned `TMPDIR` outside the exhausted `/tmp` filesystem:

- `cargo test -p molten-core fabric_time -- --nocapture`: 44 passed at the first successful post-edit run. A later added active-slot-survival regression requires the final workspace run recorded below; this count does not include it.
- `cargo test -p molten --lib fabric_time -- --nocapture`: 43 passed, including live/simulation stale callbacks, physical-slot ABA, Complete/Cancel/cleanup retention, exact-key progress after occurrence exhaustion, rejected-state equality, malformed/legacy wire denial, and matching/divergent canonical replay.
- `nix build .#checks.x86_64-linux.fabric-port-boundaries --no-link -L`: passed.
- `nix build .#checks.x86_64-linux.inherited-tracey-debt --no-update-lock-file --no-link -L`: passed; 1,933 inherited uncovered requirements remain unchanged, six source references resolve only as pending identities, zero dangling references. Pending identity is not accepted coverage.

## Source acceptance scope

At the earlier pre-review checkpoint, both required catalogued producers reported `clean`, zero findings, zero warnings and zero errors: `cargo octet check --artifact-dir target/octet` and `cargo octet check -p molten --artifact-dir target/octet-lib -- --lib`. Configuration, profile, lint selection and thresholds are unchanged. The complete source-input enumeration supplied all 1,931 `src/**/*.rs` members, including the new codec, executable-retention fixture and callback regression parts, to the object-corpus producer; it emitted 14,387 function objects. Final-source acceptance is recorded separately below.

The separately requested diagnostic `cargo octet check -p molten-core --artifact-dir target/octet-f11-core` reported `warning-only`, 1,034 findings and zero errors before the final local predicate rename. This is not passing whole-core source acceptance. Existing workspace metadata selects `molten` and `molten-node-host`; no core warning allowance or weaker source gate was added. Core behavior is checked separately from that diagnostic debt.

Initial configured producers reported source-length, borrowed-collection, channel/result-handling, bounded-growth and test-name findings. These were repaired by established source-part splits, owned bounded event storage, a bounded receiver probe, propagated delivery results and nonredundant test names; no policy was changed. Incidental error-string and immutable-input replay assertions were removed rather than re-pinned.

`cargo fmt --all --check` passed after those repairs. The first full `x86_64-linux` flake gate also passed, with 1,801 nextest tests and CI receipt `blake3:fdac2d568e1ac879caaa613659f5978377ca2fb36a311eb81e0990c950394560`. That checkpoint predates the later review-driven live-probe repair and is not substituted for final-source acceptance.

The ordered corpus/import/strict-gate/remediation sequence passed. The retained [artifact-ledger receipt](2026-10-21-artifact-ledger-receipt.preserves) is `blake3:3b890d594ca03651a7d70daa53d8eae3ac9a6068a147f466ac13c5785e5c1ac5`; the unchanged [`strict-ci` gate receipt](2026-10-21-gate-receipt.preserves) is `blake3:7d07410982c5a61df9e93c87d7469faa072e7d3524dc00a1b6d9a1e5ed3c588d`; the [remediation plan](2026-10-21-remediation-plan.preserves) is `blake3:c154d70b565bb547c360258de48ba803a9b3748d15b133dbc6ae1272f9448204`. They record bounded source-policy observations, not the missing F02 source-to-running-binary producer contract.

## Actual CLI execution

The current built binary, not a fixture mock, ran:

```sh
cargo run --bin molten -- fabric-time run-fixture --profile both --out "$OWNED_TMP/both"
cargo run --bin molten -- fabric-time show "$OWNED_TMP/both/report.preserves"
cargo run --bin molten -- fabric-time run-fixture --profile deterministic-simulation --out "$OWNED_TMP/simulation-a"
cargo run --bin molten -- fabric-time run-fixture --profile deterministic-simulation --out "$OWNED_TMP/simulation-b"
cmp "$OWNED_TMP/simulation-a/report.preserves" "$OWNED_TMP/simulation-b/report.preserves"
```

Observed two-profile output: `profile=both`, `live-domain=monotonic`, `simulation-domain=virtual`, 428 evidence events. Readback: `generation=1 final-ticks=36 timers=5 scheduler=401 entropy=3 deadline-lease=5 faults=6 live-clock=true conformance=true`.

Canonical two-profile report: `blake3:05e8584fe074aa53e1adfae98666b0ac06cf7c5f34829bd781a5111687c0c1c1`, retained as [the actual report](2026-10-21-both-report.preserves).

Both fresh deterministic runs emitted canonical report `blake3:e8e248e1a2254e82c5c4f7e8930d2ee205671af5668fd2d0c2096f7b5b038d99`. `cmp` passed. Both rendered report files have raw-byte BLAKE3 `83c8ffdcb56d2f71abd31b8f11343f9f5a3a5213b605f505ba983f541740a654`; that raw text digest is not the canonical packed-Preserves identity. The first run's [actual deterministic report](2026-10-21-simulation-report.preserves) is retained.

The fixture runs repeated same-name fresh occurrences through Complete, Cancel and generation cleanup, then submits all five evicted callbacks against fresh work. Simulation consumes actual deferred tokens; live execution probes an actual registered parked receiver. Scheduler event artifacts use `fabric-time-event-v2` and bind occurrence identity plus actual post-transition sparse state. Reports retain event references; they are not a self-contained archive of every referenced event.

After the source-policy repairs, the binary freshly built by the ordered source-gate commands ran all three actual CLI executions again, directly inside the Nix shell. Final two-profile execution/readback retained the same counts and successful conformance with report `blake3:d7b64a86506232fd953335e12910c9e56b239a7889cb7d224b0dc5358d02a185`; [the actual final report is retained](2026-10-21-final-both-report.preserves). Fresh `final-simulation-a` and `final-simulation-b` again both produced `blake3:e8e248e1a2254e82c5c4f7e8930d2ee205671af5668fd2d0c2096f7b5b038d99`, `cmp` passed, and both raw report files again had BLAKE3 `83c8ffdcb56d2f71abd31b8f11343f9f5a3a5213b605f505ba983f541740a654`. The live report is an independently observed run, not a deterministic-live claim.

## Review-driven live-probe repair

Read-only core review found no blocking law or capacity defect. Its coverage finding was resolved by direct assertions for derived terminal slots, physical slots and total slots in the existing active-admission regression. The two task-added unchecked count casts were replaced with checked conversion or matching-width constants. An incidental observation-reference prefix assertion was removed, not re-pinned.

Read-only shell/wire review identified a real teardown defect in the new live retention helper: `Ok(RejectedOverload)`, `Ok(Backpressure)` and stale-generation discard did not deliver a wake and therefore left its receiver parked at `join`. A normal real-thread regression started exactly one test against the pre-fix helper and did not finish; `timeout 12s` terminated it with exit 124 after a successful separate compilation.

The helper now observes completion through a bounded channel, releases every non-Woken outcome and timed-out observation, joins the owned thread and unregisters only its own successful registration before propagating errors. The captured-sequence equality tautology was removed. Static re-review reported the finding resolved and no new blocker; it did not execute checks and is not runtime evidence.

## Clippy size repairs

`cargo clippy --workspace --all-targets -- -D warnings` initially denied the post-review source with thirteen `result_large_err` errors in `crates/molten-core/src/fabric_time/scheduler/mod.rs`: the F11 `occurrence_sequence` field made `SchedulerError::UnexpectedReplayChoice` a 128-byte variant, so every scheduler `Result` carried it. Repaired by moving the two keys into a named `ReplayChoiceMismatch` payload behind one `Box`; only rejected replay choices allocate, and diagnostics keep both exact identities. No allow attribute, threshold change, or lint removal was used, and all pattern sites use the boxed variant.

Clippy then surfaced `large_enum_variant` in the task-owned live/simulation receiving test scaffolding (`src/fabric_time/tests/adapters/receiver.rs`): `DeliveryBoundary` carries a full post-transition snapshot, so both mailbox enums were dominated by that variant. The test-only payloads are now boxed. These are test scaffolding shapes, not runtime protocol or evidence identity changes.


After the repair, explicit formatting and `cargo fmt --all --check` passed. `cargo test -p molten --lib fabric_time -- --nocapture` passed all 44 tests, including real successful delivery, both overload policies, stale-generation discard and zero-issuance error. Denials/errors preserve full scheduler/capacity state and leave no registered target. `cargo test -p molten-core fabric_time -- --nocapture` passed all 45 tests, including exact blocked-occurrence survival during repeated terminal eviction.

The actual CLI then ran both profiles, read back the report, ran two fresh deterministic profiles and byte-compared their reports. Two-profile readback again reported 428 events, `generation=1 final-ticks=36 timers=5 scheduler=401 entropy=3 deadline-lease=5 faults=6 live-clock=true conformance=true`, with canonical report `blake3:ec64f11a087d4a32dccb1886886e9edb5179c6d222bf8f2418e817935ab506e3`. The [actual accepted report](2026-10-21-accepted-both-report.preserves) is retained. Both deterministic runs again produced `blake3:e8e248e1a2254e82c5c4f7e8930d2ee205671af5668fd2d0c2096f7b5b038d99`; `cmp` passed and both raw report digests were `83c8ffdcb56d2f71abd31b8f11343f9f5a3a5213b605f505ba983f541740a654`.

## Final-source acceptance

After the clippy repairs, `cargo fmt --all --check` passed. Clippy with `-D warnings` passed for default features, the combined optional cohort `doltlite-oracle,executable-extents,world-snapshot-vm-cohort,profiler,profiler-alloc,profiler-perf`, and `profiler-disabled`. The profiler cohorts are `x86_64-linux` development features; they establish compile and lint behavior only.

`cargo test -p molten-core fabric_time` passed 45 tests and `cargo test -p molten --lib fabric_time` passed 44. `cargo test --workspace --all-targets` passed 2,262 tests across 18 suites; `cargo test --workspace --doc` passed seven doctests; the `doltlite-oracle,executable-extents` cohort passed 2,277 tests across 19 suites with one ignored external-DoltLite test; and `world-snapshot-vm-cohort` passed 2,266 tests across 18 suites. No test failed in any run. This is default plus declared optional-cohort coverage, not unqualified all-features acceptance.

Both configured Octet producers reported `clean` with zero findings, warnings and errors on the final source: the retained [workspace status](2026-10-21-accepted-source/workspace/status.json) uses unchanged profile `b3:bdaab55d417efab9becf009dbf8e655a084da641a854331fc72bf76fab13e482`, and [library status](2026-10-21-accepted-source/library/status.json) uses `b3:2aad1f1f1a78bc9277fdb6abe27b39a261cacbd0e2d280f548667b61f852dbbd`. The native [object corpus](2026-10-21-accepted-source/workspace/object-corpus-receipt.json) selected all 1,931 explicit `src/**/*.rs` members and reports 14,388 objects over 1,607 object-bearing paths with object-set identity `b3:7c6460eef83ef8d5341788f16f7296b44be2c715f08b8332a8948c461125087d`.

Artifact intake passed with six imported artifacts, [receipt](2026-10-21-accepted-source/workspace/artifact-ledger-receipt.preserves) `blake3:af05503d03fe7e15d4fb258d0d5b3d43d292eb6f7e6c5888727e05ae7c57e883`. The unchanged `strict-ci` gate passed, [receipt](2026-10-21-accepted-source/workspace/gate-receipt.preserves) `blake3:197c4e3fb3c2eed736144c9333369012eeeb68e1eee4137b1cca332047f82346`. The [remediation plan](2026-10-21-accepted-source/workspace/remediation-plan.preserves) is `blake3:7dbae5c2a006714daadc142f3179c9b4247fc5c2c997eeae2251d500763394a5`. These record bounded source-policy observations, not the missing F02 source-to-binary producer contract.

That final source built the binary used for the last actual CLI execution. The two-profile report is `blake3:cf957d92df46292141a5b1f8c43433bbb90fb300e4d39791ed9bbb9db6008c9a` with the same 428 events and readback counts, retained as [the actual final report](2026-10-21-final-both-report.preserves). Fresh `final-sim-a` and `final-sim-b` again both produced `blake3:e8e248e1a2254e82c5c4f7e8930d2ee205671af5668fd2d0c2096f7b5b038d99`, `cmp` passed, and both raw reports again hashed to `83c8ffdcb56d2f71abd31b8f11343f9f5a3a5213b605f505ba983f541740a654`. The live profile is one independently observed run, not a deterministic-live claim.

`nix flake check -L --keep-going --no-update-lock-file` then exited 0 with `all checks passed!` for the exported `x86_64-linux` checks, including the exported Octet deny-all consumer checks, the hermetically built package, and the dogfood checks. The hermetic nextest check ran 1,802 tests with zero failures, and its canonical CI receipt is `blake3:11a3b87eb16b62325267581a718a8b4cad97ec0652150a0502073d22b9ed1e3c` with `decision=pass`; the receipt binds source ref `blake3:440d7c2281c9b66aa35b51145db992f1861d16d3bcc90f5075709d13bcf7c135`. The check omitted the incompatible `aarch64-darwin`, `aarch64-linux`, and `x86_64-darwin` systems, so no cross-platform result is claimed.

After that gate completed, the only further edits were this package's lifecycle prose and retained evidence under `.cairn/changes/bound-terminal-scheduler-retention/`; no product source, test, configuration, profile, or dependency changed. The final native validation below covers the completed lifecycle state.



## Preserved failed attempts

- Initial core test linking failed because `/tmp` had no remaining quota. Retried with task-owned temporary storage; the core tests then passed.
- Initial runnable decoder used a slice method unavailable on Preserves' record view. Corrected to the established fixed-field-count record parser; the runtime/codec tests then passed.
- Full Nix gate attempts using private-home scratch failed on traversal permissions; a `/var/tmp` alternative failed Nix's world-writable-ancestor security check. No global permissions, sandbox policy, source gate, lint baseline or check selection was weakened. Later gate outcomes must be recorded separately, not inferred from successful evaluation.
- A later configured source-producer attempt failed on a missing dependency `.rmeta` and an invalid Cargo metadata package ID. An isolated compiler target then completed checking with zero findings but failed artifact publication at the old `target/octet` root (`File exists`). These are failed attempts, not clean producer receipts. Final producers use a separate owned compiler target, one build job and fresh native artifact roots without changing policy.
- The sibling Cairn `path:` build could not import `/socket`; its Git-filtered build lacked `verified/source_review.rs`. The installed native executable is used with the explicit sibling policy, not a substituted local policy. Its immutable executable is `/nix/store/j3ni3lj66ylskpcg78ip531s9lz4z4av-cairn-0.1.0/bin/cairn`, raw BLAKE3 `2be5278a2f21f323720ba274bc96f5908a0567add051261979544e85005a6de3`; the supplied policy's raw BLAKE3 is `7a67802377005c18ebded42899e7f564f5d87087b6c14ca70464b35886f340a9`.

## Ownership and non-claims

Immediate outcome: bounded terminal retention and rejection of evicted occurrence callbacks, exercised through the real scheduler owner, adapters and CLI.

Durable contribution: maintained core/adapter/codec/replay regressions, exact occurrence identity and deterministic reclamation, with repeatable ordinary CLI commands. Maintenance owner: Molten fabric-time maintainers; Octet and lifecycle maintainers own their respective rails.

This record does not establish global liveness, fairness beyond the admitted profile, measured performance, host memory stability, whole-runtime zero allocation, arbitrary callback transfer authority, release readiness, or completion of F02. F02 remains blocked on a reviewed authoritative source-to-binary evidence producer and a normal-path real-artifact positive control. Local scratch paths are execution locators, not durable product dependencies.

## Native lifecycle acceptance

The installed native `cairn` executable at `/nix/store/j3ni3lj66ylskpcg78ip531s9lz4z4av-cairn-0.1.0/bin/cairn` (raw BLAKE3 `2be5278a2f21f323720ba274bc96f5908a0567add051261979544e85005a6de3`) ran against the explicit sibling policy export whose raw BLAKE3 is `7a67802377005c18ebded42899e7f564f5d87087b6c14ca70464b35886f340a9`.

- Strict validation passed with 37 active changes, zero issues and zero findings; [receipt-bound output](2026-10-21-final-validate.json) records `e61b4accf33726bce0f5db822563437a6e3efd2d16d95cb3f9709733b217a6c9`.
- The proposal, design and tasks gates each returned `PASS`: [proposal](2026-10-21-final-gate-proposal.json) `608c65bb6fff362ea78069729b5e502abb74bf0f2bad6cc3d369a56ea0c29d24`, [design](2026-10-21-final-gate-design.json) `a6cc4429e15234e2ca3d661f6a6741ba9fcc5278fa52a636fd836e58e149f3b8`, [tasks](2026-10-21-final-gate-tasks.json) `c10ad1e4611a5cf78dc22f20b5a779426c4105a693c3ff2ebf00df5d3c627c2a` with 14 of 14 tasks checked.
- The native [completeness review](2026-10-21-final-review-completeness.json) returned `PASS` with receipt `a11e5423f38c192d560cb37c0477cb45e97dfc098c0dda84d4be22c2e2ca43fb`. Its advisory evidence summary counts no formal, probe or property rows; that summary is not execution evidence.
- [Sync](2026-10-21-sync-receipt.json) reported `blocked=false` and merged the delta spec into `.cairn/specs/fabric-time-scheduling/spec.md`, receipt `f17ee97228ed7a953568d8506b662b42cc638fda2d8c20bda90d0b926d1cee32`. The preview receipt is `c9dae131edc312892c62ce62d36d0db64bfb7d6eac9bc6e298df7b56137526a6`.
- [Archive](2026-10-21-archive-receipt.json) reported `blocked=false`, receipt `9e0ed5e0be35c043854100c3cbe789622bd3cf28741137239ef60c68a60c6de4`; the change now lives at `.cairn/archive/1970-01-01-bound-terminal-scheduler-retention/`.
- Strict validation after archive passed with 36 active changes, zero issues and zero findings, receipt `f24bd47f624c150723d0fce2e8e0e180cf3834292f325bd2a3d49b9ce6b1ee85`.

The earlier sibling provider failures remain recorded above; using the installed executable and explicit policy is not a rebuild of the sibling provider and authorizes no F02 work, release, or distributed claim.
