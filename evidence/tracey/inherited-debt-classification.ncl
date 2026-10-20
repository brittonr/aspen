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
  observed_date = "2026-10-19",
  classification_base_revision = "9842cc6453a00ed943a866549bb4307c8502d575",
  baseline = {
    path = "evidence/tracey/inherited-debt-baseline.txt",
    blake3 = "e32ca33a172ba31f3f81c5ca26ffbfab225bc24ec3c37f8760dd95e9bab92bc1",
    entries = 1933,
  },
  inventory = {
    path = "evidence/tracey/inherited-debt-classification.tsv",
    blake3 = "17606537784a3111e86c945a9dade3f1d0b159074403f3a215d4ab1b2a6d1daf",
    summary_path = "evidence/tracey/inherited-debt-classification.md",
    summary_blake3 = "27566bc883ef104d9ba4cef222c961f2e4c04a6046b0c4cea34e25349c217528",
    entries = 1933,
    specification_groups = 37,
    source_area_groups = 108,
  },
  classes = {
    accepted_implementation_unestablished = 1933,
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
