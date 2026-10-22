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
  observed_date = "2026-10-22",
  classification_base_revision = "9842cc6453a00ed943a866549bb4307c8502d575",
  baseline = {
    path = "evidence/tracey/inherited-debt-baseline.txt",
    blake3 = "f8d79bcc696e7a3607e4de3b9a8e23e9274f55129f3ef292758fcc61d0ed2ecc",
    entries = 1934,
  },
  inventory = {
    path = "evidence/tracey/inherited-debt-classification.tsv",
    blake3 = "87d126813f3d757157ff02bc64cffaba6b7a7671bd0b30bd09aba837a361c49e",
    summary_path = "evidence/tracey/inherited-debt-classification.md",
    summary_blake3 = "56df39b875f493b22cf88552ec0c6063863f896a30066b3ff94a92583bd8e0c1",
    entries = 1934,
    specification_groups = 38,
    source_area_groups = 109,
  },
  classes = {
    accepted_implementation_unestablished = 1934,
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
