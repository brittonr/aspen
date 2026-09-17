# F12 verification

## Scope and authority

The user authorized the Molten drain and selected `origin/molten` as the base and integration target.
Legacy `origin/main` remains outside this change.
The dedicated branch is `drain/molten-retry-saturation-20260908`.
Its base is `bb6f3830e`.
The primary checkout and its staged code remain unchanged.

Molten fabric-time maintainers own the correction and its regression tests.
The public retry planner and the fabric-time fixture are the current consumers.
The change adds no dependency or port.

## Observed baseline and regression

Before arithmetic changes, the existing retry control passed.
The fabric-time adapter baseline passed 11 tests.
The added core regressions ran against unchanged arithmetic: seven selected tests passed and one failed.
The failure reported attempt 63, actual delay 0, and expected delay 128.
The fixed-delay and rejection controls passed before the correction.

The early Pueue tasks disappeared before log export.
The baseline and red results here summarize the tool output observed in this session, not retained raw logs.
Empty exports are not evidence.

After the correction, all 361 core tests and seven doc tests passed (`core-green.log.gz`).
The fabric-time suite passed 18 tests (`adapter-green.log.gz`).
The new adapter fixtures initially used the outer canonical profile reference instead of the admitted inner reference.
The corrected fixture inputs passed without a product admission change.
Core Clippy passed across all targets and features with `-D warnings` (`octet-and-core-clippy.log.gz`, final section).
Formatting and whitespace checks passed after the edits.

## Implementation and compatibility

A width guard handles attempts at or beyond `u64::BITS`.
Checked multiplication detects high-bit loss for smaller attempts.
Both paths cap the delay at the admitted maximum.
Existing attempt, jitter, generation, time-domain, and target-overflow checks remain intact.

The fixture adapter returns a fixed-size pair of canonical retry events.
The caller appends the pair only after successful admission.
Rejected plans preserve prior events.
The tests compare exact delay and deadline events, repeat the same plan, and distinguish the historical wrapped deadline by value and reference.
These observations do not prove timer execution or a complete world-replay campaign.
The existing fixed-delay coordination tests also passed in the core suite.

Profile and event shapes remain at `v1`.
Exact replay still requires the same implementation cohort.
Historical wrapped outputs must remain unchanged as evidence, not rewritten as corrected results.
The fixture now includes a separate `retry-delay` observation, so its aggregate identity changes.

## Completion blockers

The repository Octet command failed before it produced an acceptance result:

```text
error: invalid Cargo diagnostic path: capability locator `tests/../src/test/support.rs` contains `..`
```

The diagnostic comes from an existing integration-test source path.
The combined log ends with a passing core Clippy command. That exit status does not make the earlier Octet command pass.
An attempted `deterministic-core` profile also rejected the existing configuration because it lacks `core_source_scopes`.
No lint allowance, baseline, or policy relaxation was added.

The Nix gate reported two fixed-output mismatches before cancellation of the remaining builds:

- Durable Authority State revision `a0de793`: expected `sha256-uyAcvhhlnT3ZKtoZC9eSfsCgKcgbh0mntmVqJPltNUA=`, observed `sha256-++GX+wiKkcBDeXtMJnMgpByEEcuAYRgO20Pzb8TJ6pA=`.
- Bounded HTTP revision `5abc2b9`: expected `sha256-tr6/U3exQ0a+8iq7L296kJxahcBw5kKDLp9S9RQuAno=`, observed `sha256-FlNyGeN9Bo6/Dv/YqisN+5XSNerpUHM2+6jqHvxYlsA=`.

The Nix log also contains dangling trace markers before spec sync. This change adds active requirement markers, so this result is not classified as wholly pre-existing.
`nix-blocked.log.gz` retains the failed dependency observations and cancellation.
No source hash or lockfile was changed to accept unexpected bytes.

The workspace suite passed across all targets and features with exit status 0 (`workspace-tests.log.gz`).
One pre-existing test remained ignored by the suite.
Workspace Clippy also passed across all targets and features with `-D warnings` (`workspace-clippy.log.gz`).

The focused strict Octet command exited 2 with `Status: integration-failure` (`core-octet-strict.log.gz`).
Its Cargo worker panicked in `cargo-util-schemas/src/core/package_id_spec.rs:244:40`.
Zero findings from this failed run are not a passing lint result.

Native Cairn validation and proposal, design, and tasks gates passed with the explicit owner policy.
The generated receipts are `validate.json` and `gate-*.json`.
These structural gates do not prove implementation completion.

The initial review left the replay task open because it proved canonical event divergence without a replay diagnostic.
The continuation adds an executable diagnostic and positive and negative controls. See `continuation.md`.
The continuation also repairs the source-hash mismatch and diagnostic-path intake.
Strict Octet acceptance and the full Nix gate still prevent completion.
The commit attempt failed at the repository Octet hook with the same diagnostic-path error (`commit-blocked.log.gz`).
The hook was not bypassed. That initial attempt produced no commit or push.
A later fixture refactor replaces its six-argument mutable-vector interface with a five-argument fixed-size return.
No allowance was added. This source change is not a passing strict Octet result.
The final 18 fabric-time tests and full workspace Clippy passed (`adapter-final.log.gz` and `clippy-final.log.gz`).
Logs use deterministic gzip compression. `digests.blake3` binds the compressed logs and gate receipts.
The change remains active. No spec sync, archive, or publication is claimed.
