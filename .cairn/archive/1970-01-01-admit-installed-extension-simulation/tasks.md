# Tasks

- [x] [serial] Implement explicit nominal installed profiles, node/workload rows and invariant/handler bindings with bounded registry admission. r[molten.fabric_simulation.installed_extension_admission] r[molten.fabric_simulation.installed_extension_invariant_bindings]
      `admit_installed_world` (`crates/molten-core/src/fabric_simulation/installed.rs`) validates the admitted registry, node/workload rows and exact invariant/handler bindings before scheduling.
- [x] [serial] Reuse mechanism validation and the actual scheduler while preserving reference workload/oracle admission and canonical reference projections. r[molten.fabric_simulation.installed_extension_reference_preservation]
      `reference_manifest_still_requires_its_reference_semantic_oracles` passes; the collaboration world (`src/collaborationscope/simulation/world.rs`) runs on the shared scheduler and mechanism validation.
- [x] [serial] Bind both same-core witnesses, exact generations, contiguous workload and finite per-profile limits; migrate every affected caller. r[molten.fabric_simulation.installed_extension_same_core] r[molten.fabric_simulation.installed_extension_bounded_execution]
      `loaded_profile_generation_and_canonical_body_are_not_substitutable` and `selected_workload_rejects_foreign_request_profile_node_generation_and_sequence` pass; collaboration simulation calls `admit_installed_world` and federation calls the shell wrapper `fabric_simulation::admit_extension_world`, which delegates to it.
- [x] [serial] Integrate actual collaboration application/mechanism execution, independent bound invariant checks, replay and fault fixtures. r[molten.fabric_simulation.installed_extension_validation]
      `same_application_world_replays_partition_crash_conflict_and_convergence` and `actual_simulated_response_loss_quarantines_reloads_and_snapshot_admission` pass.
- [x] [serial] Add missing/wrong/duplicate invariant, foreign profile, wrong core and over-bound negative mutants, retaining original reference negatives. r[molten.fabric_simulation.installed_extension_validation]
      `installed_admission_denies_missing_wrong_duplicate_foreign_core_and_bound_mutants` passes; the reference-manifest negatives remain.
- [x] [serial] Parent executes focused/shared checks, source rules, actual consumer smoke, strict lifecycle validation and evidence review before lifecycle closure. r[molten.fabric_simulation.installed_extension_validation]
      See `evidence/2026-10-30-verification.md` and `review.md`.
