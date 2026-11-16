# Typed metadata for the grouped inherited Tracey debt classification.
let ResolvedLink = {
  requirement_id | String,
  implementation_path | String,
  verification_path | String,
}
in
let IdentityRepair = {
  old_requirement_id | String,
  new_requirement_id | String,
  accepted_specification | String,
  preserved_archives | Array String,
}
in
let ClassificationMetadata = {
  schema | String,
  observed_date | String,
  classification_base_revision | String,
  baseline | {
    path | String,
    blake3 | String,
    entries | Number,
  },
  inventory | {
    path | String,
    blake3 | String,
    summary_path | String,
    summary_blake3 | String,
    entries | Number,
    specification_groups | Number,
    source_area_groups | Number,
  },
  classes | {
    accepted_implementation_unestablished | Number,
    implemented_missing_marker | Number,
    renamed_or_replaced | Number,
    obsolete_or_invalid | Number,
  },
  resolved_links | Array ResolvedLink,
  direct_repair_manifests | Array String,
  identity_repairs | Array IdentityRepair,
  non_claims | Array String,
}
in
let metadata | ClassificationMetadata = {
  schema = "molten.inherited-tracey-debt-classification.v1",
  observed_date = "2026-11-13",
  classification_base_revision = "9842cc6453a00ed943a866549bb4307c8502d575",
  baseline = {
    path = "evidence/tracey/inherited-debt-baseline.txt",
    blake3 = "7b7eb39ded640771e4d74a79a4e1c0856f5eb0c609248538491a09b6070db93f",
    entries = 1932,
  },
  inventory = {
    path = "evidence/tracey/inherited-debt-classification.tsv",
    blake3 = "de5eea1ea869a32166bc020a1027de8350fa0e3505261203ff78673941265fe5",
    summary_path = "evidence/tracey/inherited-debt-classification.md",
    summary_blake3 = "efb519a0d8d40e2f8e2e9c3a8fb7deec48a6d10c6a42376bffdc58f92723a463",
    entries = 1932,
    specification_groups = 38,
    source_area_groups = 109,
  },
  classes = {
    accepted_implementation_unestablished = 1932,
    implemented_missing_marker = 0,
    renamed_or_replaced = 0,
    obsolete_or_invalid = 0,
  },
  resolved_links = [
    {
      requirement_id = "molten.choreography.chorus_design_reference",
      implementation_path = "src/protocol/parts/session/p012/body.rs",
      verification_path = "src/protocol/parts/session/tests/m000/p002/body.rs",
    },
    {
      requirement_id = "molten.evidence.valence_stack_adapter.docs",
      implementation_path = "crates/molten-core/src/stack.rs",
      verification_path = "crates/molten-core/src/stack.rs",
    },
    {
      requirement_id = "molten.testing.receipt_driven_traceability.coverage_derivation",
      implementation_path = "src/testing/traceability/parts/p002/body.rs",
      verification_path = "src/testing/traceability/parts/tests/p001/body.rs",
    },
    {
      requirement_id = "molten.testing.capability_context.request_binding",
      implementation_path = "src/capability_qualification/mod.rs",
      verification_path = "src/capability_qualification/tests.rs",
    },
    {
      requirement_id = "molten.coordination.rate_limit_semaphore",
      implementation_path = "src/coordination/parts/mod/p005/body.rs",
      verification_path = "src/coordination/parts/mod/tests/m000/p002/body.rs",
    },
  ],
  direct_repair_manifests = [
    "evidence/tracey/runtime-spine-direct-repairs.ncl",
    "evidence/tracey/runtime-spine-job-dag-direct-repairs.ncl",
    "evidence/tracey/runtime-spine-sam-direct-repairs.ncl",
    "evidence/tracey/runtime-spine-content-refs-direct-repairs.ncl",
    "evidence/tracey/runtime-spine-preserves-boundary-direct-repairs.ncl",
  ],
  identity_repairs = [
    {
      old_requirement_id = "molten.effects.property_tests",
      new_requirement_id = "molten.effects.handle_scope_property_tests",
      accepted_specification = ".cairn/specs/explicit-effect-handle-scoping/spec.md",
      preserved_archives = [
        "cairn/archive/2026-06-12-explicit-effect-handle-scoping",
        "cairn/archive/2026-06-13-unison-effect-handlers",
      ],
    },
  ],
  non_claims = [
    "implementation-established-for-unmapped-entries",
    "replacement-established-for-unmapped-entries",
    "obsolescence-established-for-unmapped-entries",
    "invalidity-established-for-unmapped-entries",
    "behavioral-correctness",
    "release-readiness",
  ],
}
in
metadata
