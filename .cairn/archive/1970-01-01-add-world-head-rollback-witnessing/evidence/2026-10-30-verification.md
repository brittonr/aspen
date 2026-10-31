# Verification: world-head rollback witnessing closeout (2026-10-30)

Worktree `/home/brittonr/.cargo-target/aspen-witness-20261030`, branch
`cairn/world-witness-closeout-20261030`, base `ec1f3d7f4`. Every Cargo command ran as
`env XDG_CACHE_HOME=… TMPDIR=… CARGO_TARGET_DIR=/home/brittonr/.cargo-target/aspen-witness-target-20261030
CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0 nix develop path:. --builders '' -c <cmd>`.
Only commands listed here were run; results are copied from their output.

## Baseline (before any source edit)

| Command | Result |
|---|---|
| `cargo test -p molten-core --lib world_` | ok, 147 passed, 0 failed (world-head and world-witness cores) |
| `cairn validate --strict` | `valid: true`, 0 issues |
| `cairn gate proposal\|design\|tasks add-world-head-rollback-witnessing` | each exit 0, `issues: []` |

## Focused tests after the change

| Command | Result | Log |
|---|---|---|
| `cargo test -p molten-core --lib world_` | ok, 147 passed | `2026-10-30-molten-core-world.log` |
| `cargo test -p molten --lib world_witness` | ok, 16 passed, including the three new `provider::tests::supersession` tests | `2026-10-30-molten-lib-world-witness.log` |

A first `cargo test -p molten --lib world_witness` run was lost mid-compile when the
session was interrupted by a provider API error (no test result was produced); it was
rerun from scratch with the result above.

### Rerun on the final source

After the runs above, Octet remediation inlined the reviewed contract module into
`src/world_witness/provider.rs` (removing `provider/contract.rs`) and folded the pilot's
orphan step into `session.rs`. Every result in this file was then rerun on that final
source, and the retained logs and transcripts are from the rerun:

| Command | Result |
|---|---|
| `cargo fmt --check` | exit 0 |
| `cargo check --workspace --all-targets` | exit 0, no warnings |
| `cargo test -p molten --lib world_witness` | ok, 16 passed, 0 failed |
| `cargo test -p molten-core --lib world_` | ok, 147 passed, 0 failed |
| `cargo build --bin world-witness-consumer-pilot` | exit 0 (sha256 `9143cdc5…bec452af9`) |
| both pilot campaigns below | each exit 0 with every expected step outcome |

## Consumer pilot against the real release-channel provider

`provider` built from the exact pinned Cargo checkout `fa389e1` (see
`2026-10-30-provider-decision.md`, sha256 `1e4e7fc2…a47e33`).

```
umask 077
world-witness-consumer-pilot run <tmp>/pilot-external-final-1793416059 \
  fa389e19d5c9bd84cd75df914cc5418c70d92bfb \
  --provider-program <target>/release-channel-fa389e1/debug/provider
exit=0
```

Transcript: `2026-10-30-pilot-external-provider.log`. Every request crossed a new
`provider serve INDEX CONFIG` process of that executable (`provider-program` retained in the
root; `provider-config.json` mode 0600, root and index mode 0700). Final provider state:
`2026-10-30-pilot-external-provider-final-state.json` (revision 10, highest = committed =
ordinal 4, no pending).

The same campaign with the built-in `index-serve` child (same pinned library) also exited
0: `2026-10-30-pilot-builtin-provider.log`.

Observed steps (each step must print its exact expected outcome, not only an exit status):

| Step | Expected exit | Observed |
|---|---|---|
| provision with real but unreviewed revision `3577429…` | 1 | `operator provider source pin differs from compiled reviewed contract`; no root created |
| provision with `fa389e1` | 0 | provisioned |
| advance 1, check, check | 0 | `committed generation=1 external_ordinal=1`; two independent reads `current generation=1 external_ordinal=1 external_revision=3` |
| advance 2, check | 0 | `generation=2 external_ordinal=2`, revision 5 |
| abandon (competing gen-3 reservation, orphaned; consumer never staged it) | 0 | `abandoned-orphan head_generation=3 external_ordinal=3 external_revision=7` |
| advance 3 with unresolved orphan | 1 | `unresolved external reservation or orphan blocks advance` |
| check with unresolved orphan | 1 | `external high-water rejects stale, staged or orphan claim` (not a rollback verdict) |
| resolve-orphan | 0 | `resolved-orphan external_high_water=3 external_committed=2 external_revision=8` |
| advance 3 | 0 | `committed generation=3 external_ordinal=4` (only admitted gap) |
| check | 0 | `current generation=3 external_ordinal=4 external_revision=10` |
| check with provider index unavailable | 1 | `Unavailable: capability process failed` (no downgrade) |
| restore whole consumer root to gen-1 snapshot, check | 2 | `rollback-detected local_generation=1 local_external_ordinal=1 external_ordinal=4` |

## Spec case mapping

| Case | Coverage |
|---|---|
| Valid single-provider advance, repeated read | pilot; core `admitted_checkpoint_advances_and_repeated_current_read_is_bounded` |
| Admitted quorum | core `admitted_quorum_can_grow_but_retained_witness_strength_cannot_shrink`, `maximum_witness_quorum_is_admitted_…` |
| Orphan witnessed claim / superseded | core `witnessed_orphan_cannot_move_head_after_local_contention_or_stage_replacement`; shell `orphan_resolution_preserves_highwater_…`, `supersession::resolved_orphan_high_water_is_the_only_admitted_external_gap`; pilot |
| Already complete, bounded recovery | core `exact_successful_append_recovers_to_finalization_and_complete_publication`; shell `lost_append_retains_exact_operation_…`, `supersession::lost_reserve_across_gap_recovers_the_signed_external_position` |
| Whole-store rollback | pilot (real processes); shell `whole_consumer_database_rollback_is_detected_by_fresh_authenticated_query`, `staged_claim_is_not_current_and_whole_consumer_rollback_is_rejected`; core `whole_store_rollback_and_checkpoint_fork_are_not_hidden_by_valid_signatures`; `supersession::rolled_back_predecessor_and_forged_gaps_cannot_plan_a_successor` |
| Stale checkpoint | core `StaleCheckpoint` cases (`currentness.rs`); shell `cached_signed_reply_cannot_satisfy_a_new_query_challenge`, `namespace_and_minimum_revision_are_fenced` |
| Invalid inclusion, failed consistency, split/insufficient quorum | core `invalid_proofs_quorum_currentness_and_checkpoint_are_denied`, `advance_must_extend_exact_highest_checkpoint_…` |
| Provider fork | core `unavailable_fork_and_reserved_observations_keep_distinct_outcomes`, `whole_store_rollback_and_checkpoint_fork_…`; shell `checkpoint_identity_ignores_query_nonce_but_binds_exact_state` |
| Provider substitution, wrong branch, operation substitution | core `provider_domain_generation_and_operation_substitution_are_denied`; shell `namespace_and_minimum_revision_are_fenced`; pilot unreviewed-pin rejection |
| Missing highest state | core `strongest_retained_assurance_cannot_be_reset_or_downgraded`; shell `missing_consumer_highest_is_not_reenrollment_when_head_survives` |
| Unavailable provider | core `Unavailable`; pilot unavailable step |
| Uncertain append / local commit | core `uncertain_append_and_local_commit_do_not_become_duplicate_dispatch`; shell lost-reserve tests |
| Witness-as-authority, authority denies | core `witnessed_claim_never_overrides_denied_or_stale_authority`, `recovered_append_cannot_override_current_authority_denial`; receipts `currentness_cannot_replace_authorization_or_claim_identity` |
| Local-profile overclaim, strong policy with only local state | core `local_only_evidence_cannot_publish_strong_policy_head`; receipts `strong_receipt_cannot_forget_highest_or_nonclaims` |

## Static checks and lifecycle

All on the final source (worktree commit base `ec1f3d7f4` plus this change):

| Command | Result |
|---|---|
| `cargo fmt --check` | exit 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0, no warnings |
| `cargo octet check --artifact-dir …/octet-final` | `Status: clean`, `Findings: 0` |
| `cargo octet check -p molten --artifact-dir …/octet-final-lib -- --lib` | `Status: clean`, `Findings: 0` |

Lifecycle commands ran through the sibling provider,
`nix run path:/home/brittonr/git/OnixResearch/cairn#cairn -- <command> --root .
--policy /home/brittonr/git/OnixResearch/cairn/cairn-policy/generated/cairn-policy.json`;
their JSON outputs are retained under `2026-10-30-closeout/`.

| Command | Result | Retained |
|---|---|---|
| `validate --strict` (before closeout) | `valid: true`, 0 issues | — |
| `gate proposal\|design\|tasks add-world-head-rollback-witnessing` | each exit 0, `issues: []` | `gate-*.json` |
| `sync add-world-head-rollback-witnessing` (plan) | `blocked: false`, no merge diagnostics, 7 added requirements | `sync-plan.json` |
| `sync … --execute` | `mutated: true`, merged into `.cairn/specs/world-commit/spec.md` | `sync-execution.json` |
| `archive add-world-head-rollback-witnessing` (plan, then `--execute`) | `blocked: false`; moved to `.cairn/archive/1970-01-01-add-world-head-rollback-witnessing` | `archive-plan.json`, `archive-execution.json` |
| `validate --strict` (after archive) | `valid: true`, 0 issues | `postarchive-validation.json` |
| `nix build path:.#checks.x86_64-linux.inherited-tracey-debt` | `verdict=pass`, `dangling=0` | — |
