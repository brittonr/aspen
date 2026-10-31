# Tasks

Every checked task names its implementing symbols and the executed tests or records that support
it; commands and results are in `evidence/2026-10-31-verification.md`.

## 1. Baseline and dependency admission

- [x] [serial] Record current fabric-membership, authority, resource, consistency, durable-state, logical-time, system-extension, and simulation boundaries before implementation. r[molten.collaboration_scope.versioned_extension]
      `evidence/2026-10-31-baseline-and-dependencies.md` §1.1. Reconstructed after implementation from
      `d7e435e71`, the last mainline commit without collaboration source; it is not a contemporaneous
      pre-implementation record. `git diff --stat d7e435e71 c267d062c -- src/fabric_membership` is empty.
- [x] [serial] Admit the nominal-reference and simulation dependencies, plus exact external Basalt and UCAN consumer identities or bounded blockers. r[molten.collaboration_scope.nominal_model] r[molten.collaboration_scope.authority_snapshot]
      Same record §1.2: `molten_core::nominal`, installed-extension simulation, Basalt/UCAN at
      `89675cd4f585f837323c049e4a25f7b94c903038` (`Cargo.lock`, flake input `basalt-src`).
- [x] [parallel] Record the QM source revision, reviewed mechanisms, rejected mechanisms, license facts, and non-transfer claim boundary. r[molten.collaboration_scope.validation]
      Same record §1.3: QM `7f2c916…`, fetched LICENSE (MIT, "2026 QM contributors"), README and SECURITY
      with sha256; reviewed/rejected mechanisms; no QM code or tests copied or run.

## 2. Build the functional core

- [x] [serial] Define nominal scope, subject, membership, resource-binding, audience, request, operation, policy, decision, snapshot, and receipt references with bounded validation. r[molten.collaboration_scope.nominal_model]
      `molten_core::collaboration_scope::identity` (`domains!` over `nominal::CanonicalRef`), `model::MAX_*`,
      `validate_state`; decoders reject raw values: `strict_projection_rejects_unknown_payload_and_raw_identity_fields`,
      `snapshot_decoding_rejects_payloads_raw_identities_and_bearer_reuse`.
- [x] [depends:nominal-model] Implement pure scope creation and membership add, remove, expiry, and duplicate-operation transitions with monotonic membership epochs. r[molten.collaboration_scope.membership_currentness]
      `transition::{preview_mutation, create, put_member, remove_member, advance}`; tests
      `all_scope_kinds_create_and_personal_scope_cannot_gain_foreign_members`,
      `role_changes_removal_expiry_and_duplicate_conflicts_are_epoch_fenced`,
      `hegel_membership_epochs_are_monotone_replayable_bounded_and_denial_preserving`.
- [x] [depends:nominal-model] Implement one-owner resource binding and explicit consistency-serialized scope-move transitions. r[molten.collaboration_scope.resource_binding]
      `transition::{bind, move_binding}`; committed through the Raft `CommittedBatchHandler`
      (`Application::apply_batch`); tests `exact_bind_and_explicit_move_keep_one_owner_and_floor_denials_preserve_binding`,
      `hegel_bind_and_move_keep_exactly_one_current_owner_scope`,
      `independently_signed_conflicting_mutations_are_serialized_into_one_allow_and_one_deny`.
- [x] [depends:membership-currentness] [depends:resource-binding] Implement effective-audience intersection over current membership, scope policy, resource policy, and organization-floor facts. r[molten.collaboration_scope.effective_audience]
      `admission::{admit_audience, required_policies}`; tests `every_kind_admits_exact_current_audience_and_rejects_future_use`,
      `floor_denial_wrong_bindings_expiry_revocation_and_ability_preserve_state`,
      `missing_and_duplicate_policy_facts_never_satisfy_the_intersection`.
- [x] [depends:resource-binding] Implement separate `resource/share` admission and deny inference from read, use, ancestry, or prior delivery. r[molten.collaboration_scope.nontransitive_sharing]
      `Ability::Share` with both scope-policy intersections in `admit_audience`; tests
      `sharing_requires_exact_ability_and_both_scope_policy_intersections`, `reader_cannot_share_or_implicitly_copy_a_binding`.
- [x] [depends:effective-audience] Implement exact snapshot binding, currentness validation, safe diagnostics, and deterministic BLAKE3 receipt payloads. r[molten.collaboration_scope.authority_snapshot] r[molten.collaboration_scope.safe_projection]
      `ScopeAdmissionSnapshot`, `revalidate_snapshot`, `currentness_extends`, `snapshot_ref` (canonical BLAKE3);
      tests `actual_allowed_audience_is_not_replayable_and_membership_commit_invalidates_its_snapshot`,
      `every_kind_admits_exact_current_audience_and_rejects_future_use`.

## 3. Host the system extension

- [x] [serial] Add the versioned Preserves manifest and records without changing fabric-membership schemas or treating ordinary actor traffic as collaboration traffic. r[molten.collaboration_scope.versioned_extension]
      `collaboration_scope::admit_extension` (`molten.collaboration-scope`, schema `molten.collaboration-scope.v1`);
      tests `valid_manifest_admits_one_generation_without_fabric_membership_ports`,
      `manifest_admission_denies_foreign_identity_schema_bounds_callbacks_and_ports`,
      `fabric_node_identity_without_collaboration_membership_is_denied`; fabric membership source unchanged.
- [x] [depends:functional-core] Bind scope mutations to admitted consistency, durable-state, logical-time, resource, authority, and observability ports. r[molten.collaboration_scope.membership_currentness] r[molten.collaboration_scope.authority_snapshot]
      `Installation::from_host` requires a running admitted host, its exact active consistency group and the six
      port classes (`installation_binds_only_a_running_host_and_its_exact_active_group`); `Application` commits through
      Raft and machine-loss Redb (`real_signed_mutation_is_durable_and_identical_replay_retains_original_receipt`,
      `three_participant_conflict_partition_restart_snapshot_and_currentness_recovery`, run twice).
- [x] [depends:system-extension-shell] Supply current Basalt and UCAN decision facts through the shell without storing raw tokens, keys, identity documents, or policy bodies. r[molten.collaboration_scope.authority_snapshot]
      `verify_decisions`, `NonceLedger`; tests `real_verifier_denies_invalid_signature_expiry_revocation_and_wrong_ability_before_state_effects`,
      `snapshot_decoding_rejects_payloads_raw_identities_and_bearer_reuse`.
- [x] [depends:system-extension-shell] Add bounded operator status for scope refs, epochs, counts, lifecycle, currentness, and evidence refs without member labels or resource content. r[molten.collaboration_scope.safe_projection]
      `status_value`, `Application::status`; test
      `operator_status_exposes_references_epochs_counts_and_nonclaims_without_member_or_resource_content`.

## 4. Add consumer-safe projections

- [x] [parallel] Add metadata-only snapshot fixtures for Lattice effect binding, Animus context references, and Tile read-only status. r[molten.collaboration_scope.safe_projection]
      `consumer_value` with `Consumer::{Lattice, Animus, Tile}`; test `consumer_views_are_metadata_only_and_require_revalidation`.
- [x] [parallel] Deny snapshots that contain raw tokens, identity documents, policy bodies, messages, memories, prompts, files, credentials, resource bytes, or unrestricted diagnostics. r[molten.collaboration_scope.safe_projection]
      `decode_snapshot` (closed fields, nominal positions, canonical re-encoding, non-bearer marker); tests
      `snapshot_decoding_rejects_payloads_raw_identities_and_bearer_reuse`, `strict_projection_rejects_unknown_payload_and_raw_identity_fields`.
- [x] [parallel] Document that downstream consumers must revalidate snapshot currentness and cannot treat snapshots as bearer capabilities. r[molten.collaboration_scope.authority_snapshot]
      `docs/collaboration-scope.md` "Consumer and confidentiality boundary"; the encoded marker
      `revalidate-before-effect-not-bearer-authority` is asserted by the consumer-view test.

## 5. Verify positive and negative behavior

- [x] [serial] Add positive tests for each scope kind, member changes, exact resource binding, authorized move, organization-floor narrowing, audience admission, and current snapshot consumption. r[molten.collaboration_scope.validation]
      `every_kind_admits_exact_current_audience_and_rejects_future_use`, `all_scope_kinds_create_and_personal_scope_cannot_gain_foreign_members`,
      `role_changes_removal_expiry_and_duplicate_conflicts_are_epoch_fenced`, `exact_bind_and_explicit_move_keep_one_owner_and_floor_denials_preserve_binding`,
      `sharing_requires_exact_ability_and_both_scope_policy_intersections`, `actual_allowed_audience_is_not_replayable_and_membership_commit_invalidates_its_snapshot`.
- [x] [serial] Add negative tests for stale epochs, removed or expired members, one denied audience subject, wrong scope, wrong resource, wrong ability, missing policy, and weakened organization floors. r[molten.collaboration_scope.validation]
      `floor_denial_wrong_bindings_expiry_revocation_and_ability_preserve_state` (now also wrong scope and wrong resource),
      the earlier-epoch request after removal in `role_changes_removal_expiry_and_duplicate_conflicts_are_epoch_fenced`,
      `missing_and_duplicate_policy_facts_never_satisfy_the_intersection`, `role_changes_removal_expiry_and_duplicate_conflicts_are_epoch_fenced`.
- [x] [serial] Add negative tests for transitive share, implicit copy or move, duplicate conflict, replay, revocation, partition, restart, uncertain durability, stale consumer use, and payload leakage. r[molten.collaboration_scope.validation]
      `reader_cannot_share_or_implicitly_copy_a_binding`, the two Hegel properties (conflicting replay, implicit copy),
      `real_signed_mutation_is_durable_and_identical_replay_retains_original_receipt`, `real_verifier_denies_…_revocation_…`,
      `same_application_world_replays_partition_crash_conflict_and_convergence`, `actual_simulated_response_loss_quarantines_reloads_and_snapshot_admission`,
      `actual_allowed_audience_is_not_replayable_…`, the projection and snapshot-decoding tests.
- [x] [parallel] Add properties for epoch monotonicity, one-owner binding, denial state preservation, deterministic replay, and bounded collection growth. r[molten.collaboration_scope.validation]
      `src/collaborationscope/properties.rs`: `hegel_membership_epochs_are_monotone_replayable_bounded_and_denial_preserving`,
      `hegel_bind_and_move_keep_exactly_one_current_owner_scope` (64 cases × 24 steps each); both fail against a core that
      advances two epochs or rebinds a bound resource (mutation run in the verification record).
- [x] [parallel] Add whole-system simulation and multiprocess fixtures for concurrent membership mutation, stale admission, restart, partition, and currentness recovery. r[molten.collaboration_scope.validation]
      `collaboration_scope::simulation` (installed-profile world, partition/crash/conflict/convergence, response loss) and
      `fabric_consistency::raft::collaboration_process::three_participant_conflict_partition_restart_snapshot_and_currentness_recovery`
      (three real processes over Iroh and Redb), run twice.

## 6. Validate and close out

- [x] [serial] Run formatting, focused tests, Clippy, system-extension fixtures, Cairn validation and gates, traceability, and the smallest relevant Nix checks. r[molten.collaboration_scope.validation]
      See `evidence/2026-10-31-verification.md`.
- [x] [serial] Record immutable schema, policy, Basalt, UCAN, simulation, and consumer evidence identities plus exact skipped-rail blockers. r[molten.collaboration_scope.validation]
      `evidence/2026-10-31-verification.md` "Identities" and "Not executed here".
- [x] [serial] Verify documentation preserves identity, authority, revocation, disclosure, transport, confidentiality, and release non-claims before sync and archive. r[molten.collaboration_scope.safe_projection] r[molten.collaboration_scope.validation]
      `docs/collaboration-scope.md` non-claim paragraph and evidence status, reviewed in `review.md`.
