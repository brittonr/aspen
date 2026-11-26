# Tasks

- [x] [serial] Redesign `SignedPromotionAuthority` and closure signing (`src/world_promotion/current.rs`, `crates/molten-core/src/world_promotion/`) to bind action, target, nonce/expiry over canonical packed Preserves, and validate live branch head in `src/world_promotion/operator/mod.rs`. r[molten.world_commit.promotion_authority_action_and_canonical_binding]
- [x] [serial] Make reservation+attempt transitions atomic in `src/world_promotion/service.rs` and `store/outbox.rs`, transition reservation state on `abandon_uncertain_for_attempt`, assert immutable fields in `update_reservation`, move `validate_promotion_transaction` inside `begin_write()`, and fix `read_back_promotion` partial-commit and replay handling in `store/transaction.rs`. r[molten.world_commit.promotion_atomic_dispatch_and_readback]
- [x] [serial] Fix `execute_world_head_transition` (`src/world_head/parts/service/p000/body.rs`) to emit truthful `applied`/`stale`/`uncertain` receipts and fix `p001/body.rs` to derive `role` and `authority_admitted` from signed statements and authority ports. r[molten.world_commit.world_head_truthful_receipts_and_verified_signers]
- [x] [parallel] Fix `world_merge::reduce_root` equal-root shortcut, enforce `profile.input_schema` in `planning/application.rs`, fix creation/deletion classification in `diff.rs`, and reject unreachable/detached cycles in `worldcommit/restore.rs`. r[molten.world_commit.merge_conservative_precedence_and_full_graph_closure]
- [x] [serial] Add positive/negative tests for every fixed defect, remove resolved discrepancy notes from `docs/technical/world-effects/promotion-and-reservation-atomicity.md` and `docs/technical/world-state/typed-diff-and-merge-conflicts.md`, and run focused world suites, Clippy, Octet, and strict Cairn validation. r[molten.world_commit.promotion_authority_action_and_canonical_binding] r[molten.world_commit.promotion_atomic_dispatch_and_readback] r[molten.world_commit.world_head_truthful_receipts_and_verified_signers] r[molten.world_commit.merge_conservative_precedence_and_full_graph_closure]

## Verification (2026-11-26, tasks 3–4: world head receipts/signers, world merge, world commit closure)

- Task 3: the executor returns an outcome receipt (`applied`, `already-applied`, `stale` with `StaleExpectedHead`, `uncertain`); the store is offered only the `applied` receipt (written in the head-committing transaction) and `WORLD_HEAD_UNCERTAIN_TABLE` receives the returned `uncertain` receipt. `WorldHeadSignatureCarrier` no longer carries `role`/`authority_admitted`; role comes from the trusted policy key matching the signed statement's key identity, producer, and key id (`world-head-<role>-authority-key`), and authority from the new `WorldHeadSignerAuthorityPort` (supertrait of `WorldHeadAuthorityPort`). `WorldHeadSignerObservation.role` is now `Option`. The receipt vocabulary still accepts `admitted` for `world_promotion` test fixtures that stage heads directly; the executor never emits it.
- Task 4: `reduce_root` checks runtime sensitivity, availability/schema admission, and declared mode before the equal-root shortcut (equal handler-mode roots also pass handler admission); `admitted_handler_profile` enforces `input_schema` on every present base/left/right root (`HandlerInputSchemaMismatch`); `diff_world_roots` reserves `absent` for all-three-absent roots and never reports a missing side as `equal`; `validate_parent_graph` reports `UnreachableParentObservation` and runs cycle detection from every node.
- Review fixes: the executor's in-transaction recheck re-evaluates signers through `WorldHeadSignerAuthorityPort` and re-runs pure admission over the fresh signer observations (role, signer authority, currentness, threshold), forwarding admission rather than Artifact Auth's `passed` bit (`signer_authority_withdrawn_before_commit_denies_inside_mutation_boundary`). The world-witness `authenticate` now requires exactly one signer with the trust record's role and port-admitted authority (`operator_signer_must_hold_the_trusted_role` rejects a maintainer key under recovery trust and vice versa).
- Out-of-scope files touched minimally for the API change: `src/world_witness/authority/verify.rs` (operator-trust signer authority port and role/authority enforcement), `src/cli/runtime/parts/worldhead/p000/body.rs` (document role from input), `crates/molten-core/src/world_distribution/tests/support.rs` and `src/world_distribution/parts/tests/p001/body.rs` (`role: Some(..)`).
- `cargo test -p molten-core --lib world_merge`: 16 passed. `cargo test -p molten-core --lib world_commit`: 11 passed. `cargo test -p molten-core --lib world_head`: 8 passed. `cargo test -p molten-core --lib world_distribution`: 7 passed.
- `cargo test -p molten --lib world_head`: 12 passed (after review fixes). `cargo test -p molten --lib world_merge`: 15 passed. `cargo test -p molten --lib world_witness`: 17 passed (after review fixes). `cargo test -p molten --lib world_distribution`: 6 passed. `cargo test -p molten --lib world_promotion`: 27 passed. `cargo test -p molten --test worldcommit`: 4 passed.
- `cargo clippy -p molten -p molten-core --all-targets -- -D warnings`: clean (re-run after review fixes). `cargo fmt --all` applied locally. Strict `cairn validate`: valid.
- Task 5 (whole-change validation, Octet) left unchecked for the integration owner.

## Verification (2026-11-26, world promotion tasks 1–2)

Audit claims confirmed against the source before changing it: the grant signed only
`(branch, authority, policy, generation, admitted)` as newline-joined text and the closure
comma-joined its intent refs; operator grants were compared with the reservation's historical
generation, never the live head; dispatch and recovery wrote reservation and attempt in separate
Redb transactions; abandon left the reservation `uncertain`; `update_reservation` checked only the
state transition; `validate_promotion_transaction` ran before `begin_write()` against a head read
earlier; `read_back_promotion` returned `Prior` for a partial successor and `Corrupt` once any
reservation left `committed`.

Grants are now `molten.world-promotion.authority-grant.v2` (closed action, exact target, nonce
single use per store — consumed in the applying write transaction — and wall-clock `expires_at`)
signed over `canonical_promotion_grant`; closures are `intent-closure.v2` signed over
`canonical_intent_closure`. Recovery also settles pre-change pairs (abandoned attempt with an
`uncertain`/`attempting` reservation) to `abandoned`. Docs updated:
`docs/world-promotion-current-facts.md` (including the per-store single-use non-claim),
`docs/technical/world-effects/promotion-and-reservation-atomicity.md` (resolved source-boundary
note removed). The typed-diff doc named in the last task belongs to the world-merge slice.

Commands (run on leviathan via `aremote`/`anix`, formatting locally); final run after review fixes:

- `cargo fmt --all` — clean.
- `cargo test -p molten-core --lib world_promotion` — 23 passed, 0 failed.
- `cargo test -p molten --lib world_promotion` — 39 passed, 0 failed.
- `cargo test -p molten --test worldpromotioncli --test worldpromotiondenials` — 5 + 3 passed, 0 failed.
- `cargo clippy -p molten -p molten-core --all-targets -- -D warnings` — no warnings or errors.
- `cargo octet check --artifact-dir target/octet` — Status: clean, 0 findings, 0 errors.
- `anix build -L .#checks.x86_64-linux.world-promotion-octet-deny-all` — Status: clean, 0 findings
  (first run found two `excessive_file_length` findings in the core test files; fixed by splitting
  out `tests/grant.rs` and moving one non-claims test into `tests/observation.rs`).
- `cairn validate --root . --policy …/cairn-policy.json --strict` — valid.

## Verification (2026-11-26, final validation)

Coverage was checked against the code: each fixed defect has positive and negative regressions that fail on the old behavior. No test was missing, so none was added.

1. Grant binding: `deny_or_abandon_grant_cannot_authorize_promotion`, `deny_retry_or_abandon_grant_cannot_authorize_promotion`, `current_operator_grant_admits_exactly_its_own_action_and_target`, `operator_grant_for_another_reservation_or_attempt_is_a_target_mismatch`, `expired_or_consumed_operator_grant_is_denied`, `operator_grant_must_match_the_live_branch_head`, `tampered_action_target_nonce_or_expiry_or_wrong_trusted_signer_is_denied`, `newline_and_comma_injection_cannot_collide_closure_signing_bytes`, `operator_grant_is_single_use_and_bound_to_the_live_branch_head`.
2. Atomic dispatch and read-back: `reservation_and_attempt_transition_commits_together_or_not_at_all`, `legacy_crash_between_reservation_and_attempt_writes_is_recoverable`, `deny_transitions_a_committed_reservation_and_abandon_moves_attempt_and_reservation`, `update_reservation_rejects_immutable_field_changes`, `operator_mutation_rechecks_the_validated_head_inside_its_write_transaction`, `partial_promotion_persistence_reads_back_as_corrupt`, `idempotent_replay_survives_reservations_leaving_their_initial_state`.
3. World head: `applied_stale_and_uncertain_outcomes_emit_distinct_truthful_receipts`, `local_cas_conflict_returns_stale_receipt_and_persists_nothing`, `signer_role_and_authority_come_from_trusted_policy_and_authority_port`, `signer_authority_withdrawn_before_commit_denies_inside_mutation_boundary`, `operator_signer_must_hold_the_trusted_role`.
4. Merge and commit closure: `equal_roots_do_not_bypass_runtime_sensitivity_availability_or_profile_modes`, `application_handler_enforces_declared_input_schema`, `branch_creation_and_deletion_are_classified_as_changes_not_absence`, `closure_accepts_valid_diamond_ancestry`, `closure_rejects_detached_cycle_and_unreachable_observations`.

Docs:
- `docs/technical/world-effects/promotion-and-reservation-atomicity.md` and `docs/technical/world-state/typed-diff-and-merge-conflicts.md` already described the fixed behavior. Their remaining "tracked as a defect" lines are removed.
- The two entries are removed from the discrepancy list in `docs/technical/README.md`.
- `python3 scripts/check-doc-links.py`: 320 files, 0 findings.

Commands (leviathan via `aremote`; formatting and Cairn locally):
- `cargo test -p molten-core --lib`: 823 passed, 0 failed.
- `cargo test -p molten --lib -- world_promotion world_head world_merge world_witness world_distribution worldcommit world_commit`: 97 passed, 0 failed.
- `cargo test -p molten --test worldpromotioncli --test worldpromotiondenials --test worldcommit`: 5 + 3 + 4 passed, 0 failed.
- `cargo clippy -p molten -p molten-core -p molten-node-host --all-targets -- -D warnings`: exit 0.
- `cargo octet check --artifact-dir target/octet`: warning-only, 1053 findings workspace-wide, 0 errors. Files this change touched carry 20 of them across 4 files: `world_witness/authority/verify.rs` (13, mostly `non_trait_imports`), `world_head/parts/service/p000/body.rs` (4), and `world_head/parts/tests/{p000,p001}`. They are left for OctetSweep.
- Strict `cairn validate`: valid.
- **Final task left unchecked:** the Octet source gate is not clean for this change's files (above). Every other requirement is met.

## Verification (2026-11-26, integration closure)

OctetSweep cleared the 20 findings listed above by splitting files and fixing imports, with no behavior change. These were rerun on the integration branch:
- `aremote 'cargo test -p molten-core --lib'` on `054b09732`: 834 passed, 0 failed.
- `aremote 'cargo test -p molten --lib -- … world_promotion world_head world_merge world_witness world_distribution worldcommit world_commit'` on `054b09732`: 224 passed, 0 failed.
- `aremote 'cargo test -p molten --test worldpromotioncli --test worldpromotiondenials --test worldcommit'`: 5 + 3 + 4 passed, 0 failed.
- Workspace `cargo octet check` on `6d259b9b0`: `Status: clean`, 0 findings. `aremote 'cargo clippy --workspace --all-targets -- -D warnings'`: clean.
- Strict `cairn validate`: valid.
