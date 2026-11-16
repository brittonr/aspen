## Why

Issue #45: the 2026-09-08 audit's F02 (synthetic source-gate evidence) was fixed for node startup, but `octet_gate::synthetic_clean_octet_gate_receipt_for_tests` stayed an ordinary public function. It fabricates a passing strict Octet receipt (`decision="pass"`, zero findings, refs hashing fixed `octet-test-ref` labels). `rewrites::upgrade_plan_from_apply` passed it as the plan's source gate, so `molten rewrite apply --upgrade-plan-out` satisfied the strict-source-gate requirement automatically, with initiator, capability and policy refs that were hashes of labels. Local dogfood installed the same receipt into a registry as kind `octet-gate-receipt` and handed it to node startup and job admission, where it passed as a real Octet gate.

## What Changes

- The synthetic receipt is compiled only under `cfg(test)` or the `test-fixtures` feature, which only the crate's self dev-dependency enables; library and binary builds reject any production caller.
- `apply_with_upgrade_plan` takes explicit operator refs and an `AuthenticatedGate`, which only node source-binding admission (or its `cfg(test)` fixture) constructs. It admits strict current-workspace freshness before installing replacements; the former public post-apply builder becomes private. `molten rewrite apply --upgrade-plan-out` requires `--source-state-root`, `--initiator-ref`, `--capability-ref` and `--policy-ref`, authenticates the executing source and uses this combined path. Missing, stale or unauthenticated evidence is refused before mutation.
- Dogfood binds an unqualified fixture gate (`octet-gate-fixture-v1`) that every Octet validator and source-gate consumer denies. Node startup and loopback job admission gain crate-internal fixture modes whose receipts record `strict-octet-source-gate-bound` as failed and add `unqualified-fixture-source-gate`; dogfood reports, release gates and summaries carry the same label.

## Impact

- **Files**: `Cargo.toml` (feature and self dev-dependency), the Octet gate fixture, node source binding, node runtime start, job admission, structured rewrites, the rewrite CLI, dogfood, development and operations documentation.
- **Compatibility**: `molten rewrite apply --upgrade-plan-out` now needs authenticated current source evidence and explicit refs; callers use `apply_with_upgrade_plan` with `RewriteUpgradePlanInput` rather than applying first and invoking a public post-apply builder. Dogfood report and release-gate receipts gain one check; their parsers do not require it, so archived receipts still read back.
- **Testing**: library tests cover actual replacement/plan creation and no-replacement refusal for denied, stale and fixture gate content or missing refs; every source-gate consumer denies the fixture; scan versus fixture startup and strict versus fixture job admission remain distinct. The CLI refuses invalid upgrade-plan authority without a plan or rewrite, and dogfood/CLI-harness checks retain the fixture label.
- **Owner / consumer**: evidence gates (Octet source gate), upgrade sessions, structured rewrites and the operator dogfood workflow.
