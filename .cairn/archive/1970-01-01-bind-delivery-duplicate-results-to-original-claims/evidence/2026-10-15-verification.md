# F08 verification and strict-gate blocker — 2026-10-15

## Outcome and ownership

Attempt 2/20 fixes duplicate result reconstruction: a replay returns an active
token only when its reference equals the token reference saved for the original
operation. Reclaim, completion, or lease extension can make that original token
unavailable; replay then returns no token, never a replacement consumer's token.
The original operation identity remains unchanged.

Coordination-delivery maintainers own this contract. The durable contribution is
the pure-core correction, normal core/service/reopen/record regressions, and the
consumer-facing contract in
[`docs/coordination-delivery.md`](../../../../docs/coordination-delivery.md#duplicate-result-binding).
No exported API signature, schema, historical record, completion-admission rule,
or unbounded token history was introduced. This change remains **active and not
archive-ready** because its required acceptance has not passed in full.

## Behavior and regression evidence

The recorded pre-change baseline passed 11 existing core delivery tests. The
promoted `audit_duplicate_claim_does_not_return_another_consumers_token`
regression failed before the correction: after expiry/reclaim, replay returned
consumer-b's current token instead of no token. The normal regression is kept;
no historical receipt or frozen fixture was rewritten to change that expectation.

Fresh focused execution in the dedicated Molten worktree, through its Nix shell:

- `cargo test -p molten-core --lib coordination_delivery`: 15 passed.
- `cargo test -p molten --lib coordination_delivery`: 19 passed.
- Coverage includes immediate original-token replay, expiry/reclaim, completion,
  lease-extension replacement, tokenless enqueue replay, changed-request
  conflicts, wrong owner, stale token/fencing/deadline, and exact delegated
  completion. Duplicate and denied service requests preserve the durable head,
  revision, compare-and-commit calls, timer requests, and status/worker effects.
- Actual capability-rooted Redb reopen and canonical/JSON record tests passed,
  including unavailable original tokens, original-operation binding, malformed
  token records, and truncated canonical input. Historical compatibility rows
  construct their records within the test; they are not imported pre-change
  frozen receipt fixtures.
- Independent implementation review reported no patch-introduced findings. That
  review is advisory and does not replace execution or gate acceptance.

## Executed public API smoke

A throwaway example called the real public delivery service/core APIs with the
existing controlled in-memory commit, timer, and status ports. It enqueued work,
claimed for consumer-a, expired that claim, reclaimed for consumer-b, and replayed
consumer-a's original request. The observed outcome was `duplicate-replay`, the
original operation identity, `token=None`, revision 4, and four commits. Assertions
also preserved the complete current state, commit count, and timer observations
across replay. The actual output line is retained in
[`delivery-public-smoke-output.txt`](../../../../evidence/cairn-completion-2026-10-15/delivery-public-smoke-output.txt).

The first auxiliary harness failed to compile because its nested module did not
import `DeliveryServiceStatus`; that failure is retained in
[`delivery-public-smoke-first-attempt.txt`](../../../../evidence/cairn-completion-2026-10-15/delivery-public-smoke-first-attempt.txt).
Qualifying the existing public enum corrected the harness. The subsequent
`nix develop -c cargo run --locked --example f08smoke20261015` completed
successfully. Its sole warning concerned unused variants from the included test
fixture. The temporary example was removed after execution; no product import,
lint setting, or permanent example was changed to repair that harness.

This smoke proves the supplied public API replay scenario with controlled ports,
not on-disk durability. The separate Redb reopen regressions exercise persistence.
Neither grants completion authority or proves an authority bypass, exactly-once
effects, or whole-system correctness.

## Shared acceptance results

- Default-feature `cargo test --workspace --all-targets`: 2,096 passed across
  19 test binaries, with no failed or ignored tests in those results.
- Workspace all-target checking, formatting, and Clippy with `-D warnings` passed
  in the shared acceptance run. Optional backend/feature combinations are not
  claimed by these default-feature results.
- Focused Nix acceptance passed for
  `checks.x86_64-linux.coordination-delivery-profile`,
  `checks.x86_64-linux.coordination-delivery-schema-inventory`, and
  `checks.x86_64-linux.coordination-delivery-octet-deny-all` via `nix build --no-link`.
  The deny-all check covers the existing isolated core check workspace, not all
  Molten source or all root-workspace features.
- Native proposal, design, and tasks gates passed with `valid: true` and
  `verdict: PASS`; unchanged native JSON reports are retained alongside this note
  as `2026-10-15-gate-{proposal,design,tasks}.json`. Structural gates are not runtime
  acceptance, traceability execution, or source-gate approval.

## Mandatory blocker and remaining gates

The required pinned workspace Octet producer returned `warning-only`, 1,961
warnings, and zero errors; the pinned library run returned `warning-only`, 817
warnings, and zero errors. Producer exit zero is not acceptance. The native
`strict-ci` gate denied the workspace result with:

> strict profile denies octet status `warning-only` with 1961 findings

The canonical denial receipt is
`blake3:cc2fd631e58885e89c7cb5cae295be91a6f75e56b8db63d6e40dfd03bb60750c`.
Artifact import passed; the native remediation plan was produced as
`blake3:a97d6f413479e23b271fb694291f9afbd87afcb70dbb537a7a6e9fba8fe84d99`.
Copies of these native receipts and unmodified producer statuses are retained in
[`evidence/cairn-completion-2026-10-15/`](../../../../evidence/cairn-completion-2026-10-15/).
Original producer inputs and their content ledger remain under
`target/octet-guided-goal-20261015` and
`target/octet-guided-goal-ledger-20261015` in the worktree. Receipt copies are not
a self-contained export of that ledger.

Tasks 18 and 19 stay unchecked. Compatible-feature, complete traceability/evidence,
and required repository rails are not all satisfied. No blanket `nix flake check`,
passing ordered strict source-gate sequence, runtime/release eligibility, or
zero-active-change outcome is claimed. No warning suppression, baseline widening,
weaker gate, pin change, sync, or archive was used. Further implementation awaits
the user's decision under the agreed mandatory-gate stop.
