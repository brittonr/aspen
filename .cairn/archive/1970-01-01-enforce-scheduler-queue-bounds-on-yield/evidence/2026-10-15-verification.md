# F10 adapter acceptance attempt — 2026-10-15

## Outcome and ownership

Attempt 3/20 added and exercised scheduler adapter regressions. This change is
**not complete or archive-ready**. Molten fabric-time maintainers own the
remaining scheduler/capacity/effect composition and its acceptance evidence.

The durable contribution is normal repository regression coverage in
`src/fabric_time/tests/scheduler_adapters.rs`, mounted by the existing test module.
Production scheduler semantics, dependency pins, historical receipts, and frozen
fixtures were not changed by this attempt.

## Fresh execution

From the dedicated Molten worktree, using its Nix development shell:

- `cargo test -p molten --lib fabric_time`: 34 passed, including all five new
  `scheduler_adapters` tests.
- `cargo test -p molten-core --lib fabric_time`: 41 passed.
- Default-feature `cargo test --workspace --all-targets`: 2,096 passed across
  19 test binaries; no failures or ignored tests in those results. This is not
  optional-feature coverage.
- Workspace all-target checking, formatting, and Clippy with `-D warnings`
  passed in the shared acceptance run.

The new tests cover both admitted profile kinds through the real
`ExtensionTimeContext`: accepted Yield at the active limit, fresh enqueue
positions, complete state preservation on saturated Yield under both Reject and
Backpressure, canonical action translation, stale generations, invalid source
phases, and enqueue sequence exhaustion. Denied Yield retains the original
Running occurrence. Accepted live transitions execute and observe the actual
thread-unpark adapter; a registered target refuses denied routing. Blocked-resume
controls exercise the shared Ready-admission contract with F09.

## Acceptance blockers

Task 11 remains unchecked. The tests do not establish an owning production
scheduler/capacity/effect executor or an independent deterministic-simulation
wake-effect dispatcher. `ExtensionTimeContext::apply_scheduler_command` returns
a transition; `capacity::Runtime` maintains separate reservations/accounting.
No test fabricates their missing production integration. End-to-end reservation
and effect ordering remains required; see
[`docs/fabric-time-scheduler-runtime.md`](../../../../docs/fabric-time-scheduler-runtime.md#adapter-regression-evidence-and-limits).

Task 14 remains unchecked. The required pinned workspace Octet run produced
`warning-only`, 1,961 warnings, and zero errors; the library run produced
`warning-only`, 817 warnings, and zero errors. The native `strict-ci` gate
**denied** the workspace result despite producer exit zero. Its diagnostic is
``strict profile denies octet status `warning-only` with 1961 findings``.

Canonical denial and artifact-import receipts, unchanged producer status
artifacts, and the native remediation plan are retained under
[`evidence/cairn-completion-2026-10-15/`](../../../../evidence/cairn-completion-2026-10-15/).
The gate receipt is `blake3:cc2fd631e58885e89c7cb5cae295be91a6f75e56b8db63d6e40dfd03bb60750c`;
the remediation plan is `blake3:a97d6f413479e23b271fb694291f9afbd87afcb70dbb537a7a6e9fba8fe84d99`.
The original producer inputs and content ledger remain in the worktree's
`target/octet-guided-goal-20261015` and
`target/octet-guided-goal-ledger-20261015`; the retained receipt copies are not a
self-contained export of that ledger or a passing source-gate claim.

Task 15 remains unchecked. Default-feature workspace success does not satisfy
all required compatible-feature and repository Nix rails. No blanket
`nix flake check` or scheduler-specific Nix acceptance is claimed for this run.
Native Cairn structural checks do not substitute for these behavioral gates.

No lint suppression, warning baseline widening, weaker policy, or dependency or
toolchain change was used. The mandatory-gate stop prevents new implementation
attempts pending the user's decision. Historical verification remains unchanged;
this dated note records the new execution and its narrower limits.

## Non-claims

This attempt does not prove simulated effect execution, complete capacity/effect
ordering, global liveness, measured performance, whole-runtime zero allocation,
whole-system correctness, or release eligibility. The change stays active.

## Native lifecycle checks

The native proposal, design, and tasks gates passed with `valid: true` and
`verdict: PASS`, using the documented sibling Cairn provider and its explicit
policy. The three unmodified native JSON reports are retained alongside this
note as `2026-10-15-gate-{proposal,design,tasks}.json`. They establish structural
gate observations only, not the missing runtime behavior or passing Octet/Nix
acceptance. No sync or archive was performed for this change.

