# Combined native-build and retry evidence

## Result and scope

The combined source passes both unchanged nextest profiles and the focused retry checks.
The public strict Octet gate denies acceptance. Full Nix reaches the Tracey guard and rejects six unresolved references.
Molten remains incomplete. This record does not grant lifecycle, release, deployment, or publication authority.

The merge preserves these two parents:

- Native-build, Cargo, Tracey, and strict-denial history: `55eecf8a158da4ee0db89c80f6f73175b66db99f`.
- Domain-bound retry history: `06837c6a619d0a21491dd0830119da123d2ece72`.

The dedicated branch is `completion/combined-native-retry-20260913`.
Its worktree is `/home/brittonr/.local/share/molten-completion/implementation-combined`.
A fresh fetch reported `origin/molten` at `87c289bf68c45987d2d79572de1ca458a7ca662f` before worktree creation.
The tested index tree is `5c3384ddaa223a49b5a3e047b2abdc65a3b19f8c`.
Later changes add only this evidence package and its README link.

The retry source and its lifecycle package match the retry parent.
Cargo inputs, compiler pins, nextest configuration, core crates, native tests, Tracey tools, and accepted specifications match the native parent.
The README separates historical Octet results from current acceptance claims.

The immediate outcome is one source tree with both repairs and fresh observations.
The durable contribution is a repeatable combined-source regression case, with public read-back and explicit gate refusals.
Molten maintainers own this combination and its evidence. Producer identities, runtime limits, and acceptance rules remain unchanged.

## Commands and results

Ordinary checks retain an eight-minute deadline, two Cargo jobs, and two test threads where applicable.
The existing Nix development environment supplies the normal wrapper and toolchain.
The shared target is `/tmp/molten-completion-20260913-target`.

| Task | Check | Observed result |
| --- | --- | --- |
| 1783 | Native-parent fabric-time baseline | 27 tests pass, exit 0 |
| 1814 | Combined fabric-time library tests | 29 tests pass, exit 0 |
| 1821 | `cargo nextest run --locked --test-threads 2` | 1,648 pass, zero skipped, exit 0 |
| 1823 | The same command with `--profile ci` | 1,648 pass, zero skipped, exit 0 |
| 1826 | Workspace/all-target Clippy with `-D warnings` | Exit 0 |
| 1860 | Public fixture, report read-back, event-as-report rejection, and byte comparison | Pass marker observed |
| 1861 | Configured `cargo octet check` | Exit 0, warning-only, 6,810 findings |
| 1865 | Full Nix under a separate 24-minute cold allocation | Exit 1 at the Tracey guard |
| 1938 | Complete `src` object corpus | Exit 0 |
| 1940 | Public Octet artifact import | Exit 0 |
| 1943 | Public `strict-ci` gate | Exit 1, canonical denial |
| 1944 | Formatting, metadata, and structural Cairn bundle | Outputs retained, final task result unavailable |
| 1948 | CI JUnit copy and byte comparison | Pass marker observed |
| 1981 | Checked-tree, source-manifest, and parent-scope comparison | Pass marker observed |

The default nextest build took 1m37s. Its test phase took 62.145s.
The CI build took 0.39s. Its test phase took 68.247s.
These observations are not general performance measurements.
Both profiles ran 1,648 tests across twelve binaries. Their definitions and deadlines did not change.
The focused suite retains positive, cross-domain, malformed-history, order, missing-event, and state-preservation cases.

The metadata output contains 769 packages and the four unchanged pathless Radicle package identities.
Metadata success is not all-feature test execution.
The formatter command precedes metadata and Cairn in a `set -e` bundle.
Later outputs support its successful continuation, but the final bundle result is unavailable.

## Public retry boundary

The real CLI runs `fabric-time run-fixture --profile deterministic-simulation`.
Its report has 37 evidence events, final tick 164, and `conformance=true`.
The report ref is `blake3:3fa3cce17944d1e95fd1fac660304bbfb51c8e9ec3f2dd61caf4f4569a8ff56a`.
The new report bytes exactly match the retained domain-bound report from the retry parent.

The real `fabric-time show` command accepts the report.
An event supplied as a report returns exit 1 and this diagnostic:

```text
error: invalid harness artifact: expected canonical fabric-time run report
```

This evidence does not establish live-clock, remote-timer, or production behavior.
The combined source retains exact executable-byte hashing and separate mutable test cohorts.

## Strict Octet denial

The current producer reports 6,810 warnings, zero errors, and 338 autofixable findings.
The selected compiler remains `nightly-2026-03-21-x86_64-unknown-linux-gnu`.
The profile remains `workspace-metadata`.
Its profile hash is `b3:c06e083c23147d0bc998e6d23075c50cc143e24db2dd22e320c93027b185b991`.
Its configuration hash is `b3:f1f961b27e272328cb07990eaf7801dff4e39a55623c43295277bd383e392e93`.

The complete sorted invocation supplies 1,364 Rust files under `src`.
The corpus contains 13,686 objects and 1,167 object-bearing source paths.
Its replay command exactly matches the full input inventory.
The object-set hash is `b3:cd8aceaeba30068e944f10a047ce57a91c0caf6de6a97f7d708c2744ac13cd1a`.
Object identity does not establish behavior or erase unresolved effect and dependency caveats.

The public strict receipt is `blake3:488f75312d76079258f8492b080b8fc6ad77f99cbde549c4aac5e1e46ab45e95`.
Its policy ref is `blake3:9c53411c44172b0539d16e10edf81020f870f8e86d488df09b684d0ac4e562a2`.
Only `strict-status-clean` and `no-critical-findings` fail.
Artifact presence, parsing, source scope, current metadata, and linkage checks pass.
The exact policy diagnostics are:

```text
strict profile denies octet status `warning-only` with 6810 findings
unreviewed critical octet findings: 330
```

The current critical counts are 216 `unbounded_collection_growth`, 80 `no_unwrap`, 17 `ambient_clock`, and 17 `no_panic`.
The merge adds nineteen `non_trait_imports` findings and four `path_segment_repetition` findings relative to the native snapshot.
No finding baseline, suppression, shortened inventory, synthetic clean status, or producer replacement occurs in this round.

## Nix and lifecycle boundaries

The full-Nix command keeps `--no-write-lock-file --builders '' --max-jobs 1 --cores 2`.
The separate cold allocation follows the prior dependency-compilation timeout. It does not extend an active check or runtime deadline.
Cargo 1.95 dependency compilation finishes in this run. Cache endpoint timeouts remain in the lossless log.
The command then runs thirteen Tracey guard tests successfully and rejects these unresolved references:

```text
molten.audit_f12.bounds
molten.audit_f12.compatibility
molten.audit_f12.saturation
molten.audit_f12.validation
molten.consensus.chaoscontrol_chain_observation
molten.consensus.chaoscontrol_operation_identity
```

The guard reports 2,781 definitions, 791 referenced definitions, 1,990 uncovered definitions, and 1,924 baseline entries.
Its diagnostic is `error: dangling traceability references are not permitted`.
Its derivation is `/nix/store/xv5g1m70ijmkank9is7fn5q2842xm7yz-molten-inherited-tracey-debt.drv`.
Other full-Nix obligations remain unestablished.

Structural Cairn output reports no issues across 56 active changes.
Its validation receipt is `a43c4d2895df827401eb6645f0b60d29272f4f1f4768f41633c2d4e0a27db8d2`.
The F12 and Cargo task receipts report `PASS`, but acceptance and review identifiers remain empty.
F12 retains nine checked tasks and three open tasks. Cargo retains five checked tasks and one open task.
The selected policy remains `legacy_default`, without an installation receipt.
No accepted specification changes, applied sync, lifecycle archive, or integration occurs.

## Owner-scoped review

The existing public remediation planner produced a non-accepting inventory over the native-parent artifacts.
Its receipt is `blake3:a566feb8914f22ec68f9be6080525fd9d0261a8aed7c262d542aec5aa140c865`.
That inventory does not describe fresh combined-source acceptance.

The selected Octet executable and lint library derive from `/nix/store/fmnq5i0razc5bdmncvfw331lh047100g-source`.
The evidence includes their BLAKE3 identities and Nix derivations. No tool replacement occurs.
A read-only attribution review found an existing empty-peer guard before the reported division.
It did not establish a reachable divide-by-zero defect or transfer Molten source ownership to an external producer.

A second read-only review examined eight guarded Raft panic fallbacks.
It found the proposed existing-`Result` error paths compatible at the design level.
The coordinator confirmed the exhaustive routes, existing public result type, and cloned transition state.
The proposal still needs complete message-variant fixtures, exact pre-change bytes, rejection tests, and source-gate execution.
Its error-constructor details remain unresolved in this round.
No Raft source changes occur. These correlated reviews do not provide independent approval or execution evidence.

## Preservation limits

The campaign retains direct logs, numeric exits where recorded, canonical receipts, source manifests, and JUnit data.
Before full export, eighteen task entries disappeared from the available Pueue history.
Every corresponding export attempt returned `{}`. Those files remain failed export attempts, not terminal receipts.
The affected IDs are 1783, 1784, 1787, 1814, 1821, 1823, 1826, 1849, 1860, 1861, 1865, 1869, 1915, 1938, 1940, 1943, 1944, and 1948.
Their disappearance has no established cause. No completed test reran only to replace those missing receipts.
Task 1944 lacks a retained final marker or numeric exit. Its structured outputs remain separate evidence.
The later JUnit copy and comparison succeeded through task 1948.

The first archive attempt returned exit 2 because tar treated the log globs as literal paths during creation.
Its partial raw archive, log, and exit remain separate evidence.
The corrected attempt expands those declared paths from the campaign directory and uses a new raw archive path.

`inputs.b3` binds the selected current source and configuration files.
`payloads.b3` binds this report, the input manifest, and the deterministic archive.
The archive preserves the round records, real artifacts, public output, reviews, direct logs, and failed export attempts.
Historical evidence packages keep their original source bindings. They are not rebound to this merge.
