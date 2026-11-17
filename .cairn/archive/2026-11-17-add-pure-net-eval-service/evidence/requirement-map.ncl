# Human-reviewed requirement/case map; execution facts live in the adjacent
# qualification record. Case declarations alone are not a passing execution,
# release authority, root-runtime adoption, or cross-host qualification.
{
  schema = "aspen.pure-net.requirement-evidence-map/v1",
  maintenance_owner = "Molten contributors; pilots/orchestration",
  workload = "pure-net-eval/v1",
  producer = "f7af9e972aa2968d22978794a667d267cf2745e8",
  execution_record = "qualification-2026-11-17.json",
  commands = {
    source = "nix develop -c python3 scripts/test_vendored_sources.py",
    source_lock = "nix develop -c python3 scripts/vendored-sources.py check --root .",
    producer = "nix develop -c cargo test --manifest-path pilots/orchestration/Cargo.toml -p molten-pure-net --lib --locked",
    adapters = "nix develop -c cargo test --manifest-path pilots/orchestration/Cargo.toml -p molten-orchestrator-adapters --lib --test pure_net_contracts --test pure_net_runtime --locked",
    process = "nix develop -c cargo test --manifest-path pilots/orchestration/Cargo.toml -p molten-orchestrator-cli --test pure_net_e2e --locked",
  },
  requirements = [
    {
      requirement_id = "molten.pure_net.profile",
      positive = ["process:pure_net_cli_receipt_binds_exact_objects_and_repeats"],
      negative = ["process:pure_net_wrong_identity_missing_grants_and_revocation_deny_success", "adapters:wasm_adapter_rejects_native_even_when_program_bytes_are_a_valid_component"],
      evidence_classes = ["actual-same-host-process", "deterministic-adapter"],
    },
    {
      requirement_id = "molten.pure_net.source_intake",
      positive = ["source_lock:exact-selected-tree", "source:test_real_git_import_excludes_ungranted_asset_from_bytes_and_lock"],
      negative = ["source:test_changed_byte_is_rejected", "source:test_revision_drift_is_rejected", "source:test_renaming_an_ungranted_asset_does_not_admit_it"],
      evidence_classes = ["deterministic-filesystem-fixture", "exact-source-byte-check"],
    },
    {
      requirement_id = "molten.pure_net.identity",
      positive = ["process:pure_net_cli_receipt_binds_exact_objects_and_repeats", "producer:repeated_full_binding_has_identical_canonical_receipt"],
      negative = ["producer:untrusted_checked_ir_claim_is_reconstructed_not_believed", "adapters:native_materialization_rejects_wrong_kind_or_same_kind_identity_substitution_before_begin", "process:pure_net_wrong_identity_missing_grants_and_revocation_deny_success"],
      evidence_classes = ["deterministic-producer", "deterministic-adapter", "actual-same-host-process"],
    },
    {
      requirement_id = "molten.pure_net.producer",
      positive = ["producer:complete_frozen_corpus_matches_independent_source_ir_and_exact_expectations", "producer:producer_erase_rule_fires_and_fuel_zero_never_returns_a_prefix"],
      negative = ["producer:real_validator_rejects_open_cyclic_and_wrong_ordered_nets", "producer:zero_active_pair_net_still_requires_complete_ordered_readback", "producer:untrusted_checked_ir_claim_is_reconstructed_not_believed"],
      evidence_classes = ["deterministic-producer"],
    },
    {
      requirement_id = "molten.pure_net.quanta",
      positive = ["adapters:unrelated_runnable_completes_between_real_native_quanta", "producer:wide_queue_precommit_reservations_replay_without_becoming_interactions"],
      negative = ["producer:fuel_boundary_preserves_cumulative_work_and_never_returns_partial_values", "producer:wide_queue_can_cancel_before_any_expensive_precommit_scan", "adapters:exhausted_native_job_releases_the_only_permit_for_a_valid_sibling"],
      evidence_classes = ["deterministic-producer", "actual-local-async-runtime"],
    },
    {
      requirement_id = "molten.pure_net.bounds",
      positive = ["producer:measured_setup_validation_readback_caps_are_inclusive", "producer:program_and_ordered_input_byte_bounds_are_inclusive", "producer:exact_source_bytes_limit_is_inclusive"],
      negative = ["producer:each_nonreduction_work_phase_has_an_effective_negative_cap", "producer:wide_queue_validation_and_total_caps_stop_before_the_next_atomic_commit", "producer:materialized_wide_queue_atomic_cap_is_checked_before_plan_compilation", "adapters:concurrent_native_limit_rejects_the_next_evaluation_without_starting_it", "adapters:expired_completion_cannot_publish_when_the_timeout_branch_has_not_run", "adapters:unlimited_missing_duplicate_or_weaker_caps_never_admit_native_work"],
      evidence_classes = ["deterministic-producer", "deterministic-process-limit-parser", "actual-local-async-runtime"],
    },
    {
      requirement_id = "molten.pure_net.outcomes",
      positive = ["producer:complete_frozen_corpus_matches_independent_source_ir_and_exact_expectations"],
      negative = ["producer:zero_active_pair_net_still_requires_complete_ordered_readback", "process:pure_net_typed_exhaustion_retains_failed_receipt_without_success"],
      evidence_classes = ["deterministic-producer", "actual-same-host-process"],
    },
    {
      requirement_id = "molten.pure_net.lifecycle",
      positive = ["process:pure_net_cli_receipt_binds_exact_objects_and_repeats", "process:pure_net_kill_wait_reopen_all_worker_boundaries_preserve_one_authoritative_completion"],
      negative = ["adapters:unknown_actual_output_persistence_keeps_ownership_and_no_second_ticket", "adapters:stopped_failure_receipt_is_copied_before_controller_terminal_and_unknown_retains_capacity", "adapters:revoked_retry_cannot_starve_a_later_authorized_run", "adapters:denied_first_task_does_not_starve_authorized_sibling_in_same_run"],
      evidence_classes = ["deterministic-adapter", "actual-same-host-process-restart"],
    },
    {
      requirement_id = "molten.pure_net.authority",
      positive = ["adapters:file_admission_separates_native_grants_and_rechecks_each_revocation", "process:pure_net_cli_receipt_binds_exact_objects_and_repeats"],
      negative = ["process:pure_net_wrong_identity_missing_grants_and_revocation_deny_success", "process:pure_net_revocation_after_calculation_cannot_publish", "process:pure_net_cancel_after_calculation_wins_over_late_success", "adapters:actual_component_cancel_between_calculation_storage_and_terminal_never_publishes_success"],
      evidence_classes = ["deterministic-adapter", "actual-same-host-process"],
    },
    {
      requirement_id = "molten.pure_net.recovery",
      positive = ["process:pure_net_kill_wait_reopen_all_worker_boundaries_preserve_one_authoritative_completion"],
      negative = ["adapters:unknown_actual_output_persistence_keeps_ownership_and_no_second_ticket", "adapters:old_snapshot_cannot_be_silently_replaced_by_a_new_empty_ledger"],
      evidence_classes = ["deterministic-adapter", "actual-same-host-process-restart"],
    },
    {
      requirement_id = "molten.pure_net.replay",
      positive = ["producer:repeated_full_binding_has_identical_canonical_receipt", "process:pure_net_cli_receipt_binds_exact_objects_and_repeats", "process:pure_net_frozen_matched_service_baseline_retains_unfavorable_and_retry_costs"],
      negative = ["producer:repeated_full_binding_has_identical_canonical_receipt:changed-ordered-input-and-manifest-limit", "producer:malformed_input_bounds_and_wrong_host_are_admission_failures"],
      evidence_classes = ["deterministic-producer", "actual-same-host-process"],
    },
    {
      requirement_id = "molten.pure_net.baseline",
      positive = ["process:pure_net_frozen_matched_service_baseline_retains_unfavorable_and_retry_costs"],
      negative = ["process:pure_net_frozen_matched_service_baseline_retains_unfavorable_and_retry_costs:preserved-unfavorable-results-and-no-adoption-claim"],
      evidence_classes = ["diagnostic-matched-service-baseline", "separately-instrumented-worker-lifetime"],
    },
    {
      requirement_id = "molten.pure_net.evidence",
      positive = ["source_lock:exact-selected-tree", "process:pure_net_cli_receipt_binds_exact_objects_and_repeats", "process:pure_net_kill_wait_reopen_all_worker_boundaries_preserve_one_authoritative_completion"],
      negative = ["source:test_renaming_an_ungranted_asset_does_not_admit_it", "process:pure_net_wrong_identity_missing_grants_and_revocation_deny_success"],
      evidence_classes = ["exact-source-byte-check", "actual-same-host-process", "actual-same-host-process-restart"],
    },
  ],
  non_claims = [
    "Case declarations and suite exit zero are not standalone execution or release authority.",
    "Negative rows name expected-denial/boundary assertions inside passing tests; they do not claim the Cargo process exited with a denial code.",
    "Same-host process restart is not machine-loss/NVMe durability or cross-host qualification.",
    "The frozen producer performance NO-GO remains; no default, parallel or portable-checkpoint adoption follows.",
    "Memory/allocations/timing are diagnostic observations in their declared domains, never canonical execution authority.",
  ],
}
