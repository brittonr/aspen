# Inherited Tracey debt: archived preflight requirements

## Finding

The 2026-10-19 archive batch moved nine preflight packages into
`.cairn/archive/` and merged their delta specs into accepted `.cairn/specs/`.
Their requirements have no `r[impl]` or `r[verify]` source markers and were not
added to the reviewed inherited-debt baseline, so
`checks.x86_64-linux.inherited-tracey-debt` reported nine
`unexpected_missing` identities.

The affected requirements are:

- `molten.chunk_store.caller_built_manifest_count_preflight`
- `molten.chunk_store.remote_supplied_chunk_preflight`
- `molten.consensus.placement_admitted_member_preflight`
- `molten.resources.placement_constraint_preflight`
- `molten.resources.placement_target_property_contract_missing`
- `molten.resources.reconcile_required_effect_intent_preflight`
- `molten.testing.cluster_harness_fixture_file_preflight`
- `molten.testing.receipt_first_cluster_harness.run_index_intake_bound`
- `molten.wasm_performance.phase_group_count_preflight`

## Repair

The nine accepted, implementation-unestablished requirements are recorded in
`evidence/tracey/inherited-debt-baseline.txt`. The baseline metadata,
classification inventory, and generated JSON were regenerated from their
Nickel sources and the classifier binary. No source marker was added and no
requirement was claimed as implemented.

## Verification

- Guard: `requirements=2834 referenced=901 uncovered=1933
  baseline_entries=1933 dangling=0 verdict=pass`.
- Classifier: `classified_entries=1933
  accepted_implementation_unestablished=1933 specification_groups=37
  source_area_groups=108 verdict=pass`.
- `nix build .#checks.x86_64-linux.inherited-tracey-debt`: pass.

## Non-claims

This records inherited debt; it does not establish implementation, behavioral
correctness, coverage, release readiness, or whole-system correctness for the
nine requirements.
