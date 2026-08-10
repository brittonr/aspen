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
  identity_repairs | Array IdentityRepair,
  non_claims | Array String,
}
in
let metadata | ClassificationMetadata = {
  schema = "molten.inherited-tracey-debt-classification.v1",
  observed_date = "2026-08-10",
  classification_base_revision = "264294ba091b2e9d1c777bc6728cc64a1ce876c7",
  baseline = {
    path = "evidence/tracey/inherited-debt-baseline.txt",
    blake3 = "334360f765acce463b56573883443a93a0b6764c01e82b36b90594c31d854ff0",
    entries = 1979,
  },
  inventory = {
    path = "evidence/tracey/inherited-debt-classification.tsv",
    blake3 = "d841aaa7b54e36c40a6c4dfd5f743f180ea319ee05380a65786a56115fa95849",
    summary_path = "evidence/tracey/inherited-debt-classification.md",
    summary_blake3 = "c04ade25d44ea7add725c723a1cd124afc128ef6a53547edd64e37b3fda43277",
    entries = 1979,
    specification_groups = 35,
    source_area_groups = 112,
  },
  classes = {
    accepted_implementation_unestablished = 1979,
    implemented_missing_marker = 0,
    renamed_or_replaced = 0,
    obsolete_or_invalid = 0,
  },
  resolved_links = [
    {
      requirement_id = "molten.choreography.chorus_design_reference",
      implementation_path = "src/protocol/parts/session/p009/body.rs",
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
  identity_repairs = [
    {
      old_requirement_id = "molten.effects.property_tests",
      new_requirement_id = "molten.effects.handle_scope_property_tests",
      accepted_specification = "cairn/specs/explicit-effect-handle-scoping/spec.md",
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
