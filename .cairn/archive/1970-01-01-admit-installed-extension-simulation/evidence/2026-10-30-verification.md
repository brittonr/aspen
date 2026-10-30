# Admit installed extensions to simulation — verification

## Focused executed evidence

- `collaboration_scope::simulation::world::admissiontests::reference_manifest_still_requires_its_reference_semantic_oracles`
- `collaboration_scope::simulation::world::admissiontests::loaded_profile_generation_and_canonical_body_are_not_substitutable`
- `collaboration_scope::simulation::world::admissiontests::selected_workload_rejects_foreign_request_profile_node_generation_and_sequence`
- `collaboration_scope::simulation::world::installed_admission_denies_missing_wrong_duplicate_foreign_core_and_bound_mutants`
- `collaboration_scope::simulation::same_application_world_replays_partition_crash_conflict_and_convergence`
- `collaboration_scope::simulation::actual_simulated_response_loss_quarantines_reloads_and_snapshot_admission`

The federation consumer exercises the same admission path on every live receiver run through
`fabric_simulation::admit_extension_world` (see
`../federated-pull-system-extension/evidence/2026-10-30-live-rails.md`).

## Integrated source checks (executed 2026-10-30)

Run in the integration worktree `cairn/finish-open-changes-20261029` (base
`d7e435e711f7f58167900ce7419121390b0842d3`) through `nix develop` (`nightly-2026-05-26`):

| Check | Result |
|---|---|
| `cargo fmt --check` | clean |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 |
| `cargo clippy -p molten --all-targets --features spacewasm-pilot -- -D warnings` | exit 0 |
| `cargo clippy -p molten --all-targets --features component-rpc-telemetry -- -D warnings` | exit 0 |
| `cargo octet check --artifact-dir …` | `Status: clean`, 0 findings |
| `cargo octet check -p molten --artifact-dir … -- --lib` | `Status: clean`, 0 findings |
| `cargo test --workspace --no-fail-fast` | 2,907 passed, 0 failed, 28 ignored |
| `nix build .#checks.x86_64-linux.inherited-tracey-debt` | pass, `dangling=0` |
| `cairn validate --root . --policy ../cairn/cairn-policy/generated/cairn-policy.json --strict` | `valid: true` |

The pinned Octet producer used by the startup source binding (cargo-octet 0.1.0 on
`nightly-2026-03-21`, Nix cohort) also reported `Status: clean` for the primary and library
cohorts. The 28 ignored tests require independently provisioned signed producer outputs or
external cohorts and are reported only where they were explicitly executed.

## Non-claims

Admission validates loaded identities, linkage and finite bounds. It proves neither handler
correctness nor production admission.
