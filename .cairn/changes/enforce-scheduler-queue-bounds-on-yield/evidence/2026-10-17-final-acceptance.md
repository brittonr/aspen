# F10 final bounded acceptance attempt — 2026-10-17

## Outcome and maintenance owner

Attempt 20/20 repairs only the reported collapsible fairness guard through the
Rust-analyzer code action, without changing its short-circuit decision or adding
lint allowances. Source binding confirms that this is the sole changed member
of the retained attempt-19 source corpus. Molten fabric-time maintainers own the
production scheduler/capacity/effect composition and its regression fixtures.

The owning runtime and independent simulation delivery implemented in attempts
18–19 are exercised by this run. This replaces the earlier missing-composition
blocker; it does not establish the unfinished assignment-token work, global
liveness, measured performance, whole-system correctness, or release eligibility.

## Executed acceptance

Tracked raw command records, source identities, receipts and actual CLI artifacts
are retained in
[`attempt-20-final-scheduler-acceptance`](../../../../evidence/cairn-source-remediation-2026-10-15/attempt-20-final-scheduler-acceptance/).

- Focused fabric-time tests: 41 core and 37 shell passed, with no failures.
- Both catalogued Octet producers: clean, zero findings, unchanged profile and
  configuration identities. No warning baseline or weaker gate was introduced.
- Complete 1,901-member object corpus, artifact import and remediation plan ran.
- The unchanged `strict-ci` source gate passed as
  `blake3:e50185287ffabbe677edc8207d1747ac4a3d731c9de6be3d9ff5e7e730d18ffa`.
- Workspace all-target tests: 2121 passed;
  workspace doctests: 7 passed.
- Compatible enabled-feature tests: 2140 passed,
  1 ignored. The ignored external-cohort case is not executed evidence.
- Profiler-disabled tests: 2121 passed.
- Workspace all-target Clippy with `-D warnings` passed for default, compatible
  enabled-feature and profiler-disabled configurations. Formatting passed.
- The freshly built real CLI ran `fabric-time run-fixture --profile both` and
  `fabric-time show`: seven scheduler steps, observed live clock, and conformance
  passed. The report is
  `blake3:3e399a4e79d6e8b92a60a7eca98ef3e1d07f942dd33ebf48473b405468f7f8df`.

These commands and retained normal repository regressions provide repeatable
bounded scheduler admission and delivery evidence, not a broader runtime claim.

## Environment recovery and remaining acceptance

Two ordinary Nix shell launches timed out before the formatter executed while
waiting for the shared garbage-collector lock. The existing Nix-generated direnv
cache recovered Cargo/Octet execution; its compiler output and recorded shell and
target-directory values match attempt 19, and the producer metadata also match.
No host GC settings, shared Cargo-cache entries, dependency pins, or unrelated
processes were changed.

The required Nix build of `nextest`, `clippy`, and `nextest-config` reached its
3,500-second deadline waiting for the shared garbage-collector lock. The recorder
terminated its owned command; `ordered-relevant-nix-checks.json` retains
`timed_out: true`, exit 1, and the original output. This is not passing Nix
acceptance or a behavioral test failure. The repository `nix flake check` did not
execute because the ordered campaign stopped at this failed prerequisite.

Native artifact gates and subsequent strict-validation observations are recorded
separately in the same evidence directory; they cannot replace the required Nix
acceptance. The change remains active, with its Nix task unchecked. No sync,
archive, completion commit, or remote push was performed in this attempt.

All 20 authorized implementation attempts are consumed. Further implementation
requires an extended allowance; host GC/space recovery must not be inferred as
authorized by the source acceptance evidence.
