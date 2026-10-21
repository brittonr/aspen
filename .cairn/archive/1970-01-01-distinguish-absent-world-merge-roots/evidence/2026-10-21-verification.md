# F13 root-absence execution evidence — 2026-10-21

## Scope and ownership

Change: `distinguish-absent-world-merge-roots`, on isolated worktree branch `cairn/next-audit-20261021`, based on `5f5e38139`. Immediate outcome: absence, selected existing roots, and generated data are distinct world-merge outputs, so publication no longer invents content for an absent root. Durable contribution: a closed output contract, a v3 plan identity, and maintained core/publication regressions. Molten world-merge maintainers own the core output contract, publication shell, canonical encoder, and compatibility corpus.

## Executed pre-fix reproduction

A task-owned `examples/f13_before.rs` used only the public `prepare_world_merge_plan` and `publish_world_merge` APIs with local recording ports. Both source roots and the base were absent. Output at the pre-fix revision:

```text
schema=false selected=false values=0 bytes=0 output_schema=false
schema=false publish=denied generated_calls=0 commit_calls=0 error=invalid harness artifact: generated merge root requires an output schema
schema=true selected=false values=0 bytes=0 output_schema=true
schema=true publish=ok generated_calls=1 commit_calls=1 output_roots=1
```

This reproduces both audit variants: without schema metadata publication fails, and with schema metadata it generates an empty root and commits. The example was removed after the observation; the permanent regressions below retain the corrected assertions.

The recorded pre-edit baseline ran `cargo test -p molten-core world_merge` (11 passed) and `cargo test -p molten --lib world_merge` (12 passed).

## Implementation

- `WorldMergedRoot` now carries `WorldMergeOutput::{Absent, Selected(WorldRootRef), Generated(WorldMergeGenerated)}`. `WorldMergeGenerated` requires a schema reference, so a generated output without schema metadata is unrepresentable.
- Absence is admitted only for non-runtime-sensitive kinds whose profile declares `IdenticalOnly` or `AncestorReplacement`, and only when both source schemas agree. Undeclared kinds deny with `ModeNotDeclared`, content-producing modes deny with `UnsupportedAbsence`, runtime-sensitive kinds deny with `RuntimeSensitiveRoot`, mismatched absent metadata denies with `IncompatibleSchema`, and a keyed output without a present schema denies with `MissingOutputSchema`.
- Publication matches the three cases: absent roots are omitted from the new inventory, selected roots are reused exactly, and only generated roots call `persist_generated_root`. Absence never requests a schema, writes content, or authorizes deletion.
- The canonical plan schema is `molten.world-merge-plan.v3`, with a `molten-world-merge-plan-v3.preserves` boundary marker. Plan identity moves to `world-merge-plan-identity-v3`, derives from an explicit `absent`/`selected`/`generated` case tag plus case-specific fields, and reports `PlanIdentityMismatch` for a stale or mutated plan. There is no v1/v2 reader or automatic upgrade; old plans require re-planning.

## Executed focused checks

Inside the repository Nix shell with an owned `TMPDIR`:

- `cargo test -p molten-core world_merge`: 13 passed, zero failed. Includes absent admission/denial rules and distinct absent-versus-generated-empty identity.
- `cargo test -p molten --lib world_merge`: 15 passed, zero failed. Includes absent-only publication with and without schema metadata, mixed absent/generated inventory, generated-persistence failure without commit, and the earlier conversion/tamper corpus.
- Corrected public-path observation after the change, using the same temporary public-API example before removal:

```text
schema=false case=absent
schema=false publish=ok generated_calls=0 commit_calls=1 output_roots=0
schema=true case=absent
schema=true publish=ok generated_calls=0 commit_calls=1 output_roots=0
```

- `cargo fmt --all --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test --workspace --all-targets` passed 2,267 tests across 18 suites with zero failures; `cargo test --workspace --doc` passed 7 doctests.

## Source acceptance

Both configured Octet producers reported `clean` with zero findings, warnings and errors under unchanged profiles: [workspace status](2026-10-21-accepted-source/workspace/status.json) (`b3:bdaab55d417efab9becf009dbf8e655a084da641a854331fc72bf76fab13e482`) and [library status](2026-10-21-accepted-source/library/status.json) (`b3:2aad1f1f1a78bc9277fdb6abe27b39a261cacbd0e2d280f548667b61f852dbbd`).

The native [object corpus](2026-10-21-accepted-source/workspace/object-corpus-receipt.json) selected every `src/**/*.rs` member and reports 14,391 objects over 1,607 object-bearing paths with object-set identity `b3:4cfa26c1951d609af37c6cac137d8fa14da31533569653eb78014a63b6f83905`; the core split does not change this shell source selection.

Ordered artifact acceptance:

- Artifact import passed with six artifacts, [receipt](2026-10-21-accepted-source/workspace/artifact-ledger-receipt.preserves) `blake3:3a084343f6011dcf346319002aa53649957cc2a1d5f77ea1190bf614dcc2cde2`.
- Unchanged `strict-ci` gate passed, [receipt](2026-10-21-accepted-source/workspace/gate-receipt.preserves) `blake3:aaa4fafca43a5fb4a36fb98564810ff2e5952e09c20ab69f8780b95225ea11ec`.
- [Remediation plan](2026-10-21-accepted-source/workspace/remediation-plan.preserves) `blake3:46c3b2f9daf7c0e3dc5a928c885f7ebf265fdf45a935dd14281fa24a82a716bb`.

The focused `world-merge-octet-deny-all` Nix check separately reported `clean`, zero findings, zero warnings and zero errors under its unchanged `b3:2af1b340…` profile. An earlier attempt on the same source was denied with two `excessive_file_length` findings after `admission.rs` grew to 336 lines; absence and keyed reduction moved into `planning/absence.rs` and `planning/keyed.rs`, leaving `admission.rs` at 265 lines. A later attempt failed with module-not-found errors because the new files were untracked and therefore absent from the Nix git source; staging them resolved the source selection without changing any check.

`nix flake check -L --keep-going --no-update-lock-file` then exited 0 with `all checks passed!` for the exported `x86_64-linux` checks, including the focused merge Octet consumer check, the hermetically built package, and the dogfood rails. The hermetic nextest check realized 1,805 testcases with zero failures; its [CI receipt](/nix/store/1spxrbaiqkj5n8cvx7hplklqhvg6pz1q-molten-nextest/ci-test-run-receipt.preserves) records `decision="pass"`, bound source ref `blake3:32590f8f603318d6bb5b68122d443f5fcfe831fbf9d8d5dad53880aa0f5b7aa3`, and raw receipt-file BLAKE3 `1cdd2d790fbbbace3846714533bcc4d755db46c40e18e19682a1853d87497df5`. That raw digest is not the canonical receipt reference. The check omitted `aarch64-darwin`, `aarch64-linux`, and `x86_64-darwin`, so no cross-platform result is claimed.

## Preserved limits

- The change does not prove arbitrary merge semantics, migration semantic equivalence, runtime-sensitive merge support, whole-system correctness, or release eligibility.
- An absent root in a new inventory is not deletion authority; retained snapshots, replay capsules, and retention holds remain separate protections.
- The pre-fix reproduction and corrected observation are bounded public-API executions with recording ports, not live-storage evidence.
- No legacy plan reader exists, so "legacy rejection" is established as the absence of any upgrade path plus identity mismatch for mutated in-memory plans, not as a decoded-v2 negative execution.

## Native lifecycle acceptance

The installed native `cairn` executable (raw BLAKE3 `2be5278a2f21f323720ba274bc96f5908a0567add051261979544e85005a6de3`) ran against the explicit sibling policy export (`7a67802377005c18ebded42899e7f564f5d87087b6c14ca70464b35886f340a9`).

- [Strict validation](2026-10-21-validate.json) passed with 36 active changes, zero issues and zero findings; receipt `249f932b2aa16a19f8f8d6b5d088e7cdb5bb0e6638866aaa2f1ebfd6b17ee537`.
- Proposal, design and tasks gates each returned `PASS`: [proposal](2026-10-21-gate-proposal.json) `f9b00fd7f7f2837df98a495468b8fe21be8ca4e5b574d47cfa09717860cf11c1`, [design](2026-10-21-gate-design.json) `423577a62fa8c0b92b0836ee625e9cbd32ec1e29f133d0aab1c50e9440d9b3a8`, [tasks](2026-10-21-gate-tasks.json) `eac9f39adb9018879602dd106b5b2abd02378ff32a9573fe034c8253597fdf47` with zero unchecked tasks.
- The [completeness review](2026-10-21-review-completeness.json) returned `PASS` with receipt `6a8f6947285b35aee3167935d74c2d9b8cbd99acc5dd46bd2821665d50dc9ebd`.
- [Sync](2026-10-21-sync-receipt.json) merged the delta spec into `.cairn/specs/world-commit/spec.md` with `blocked=false`, receipt `9b7360baaf7ad6c64b5abb50c893ec48ca7c1cab152761b9214eeeb47b65cf7f`.
- [Archive](2026-10-21-archive-receipt.json) reported `blocked=false`, receipt `3fb5d6ab3cf88e6d0017af5814b4c35aaba6d1f80adb262cf74b3220b6f813d0`; the change now lives at `.cairn/archive/1970-01-01-distinguish-absent-world-merge-roots/`.
- Post-archive strict validation passed with 35 active changes, zero issues and zero findings, receipt `b2bd8432f31ed291270fa52e39380f9c4fb7257211469b2d03d838686baea3de`.

These structural results do not replace the executed behavior evidence above and grant no release or deletion authority.
