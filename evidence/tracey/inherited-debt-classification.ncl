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
  observed_date = "2026-11-26",
  classification_base_revision = "bfb7a7f4a1d19bc81a11624d019e7c183d5fdadc",
  baseline = {
    path = "evidence/tracey/inherited-debt-baseline.txt",
    blake3 = "c46db3de2b17f02e0aa8ef5622ce97ea21bce2e393c119484a451b3408d1a3d3",
    entries = 2460,
  },
  inventory = {
    path = "evidence/tracey/inherited-debt-classification.tsv",
    blake3 = "8633653b9f9de1a9f83c61f5ab1b612dae08eb2b85c38738a7aa8c587ba5ab4e",
    summary_path = "evidence/tracey/inherited-debt-classification.md",
    summary_blake3 = "f754baafbaa9bfec6e9e55111dec1ce629800d274b4a2e24c03e366bd7a35458",
    entries = 2460,
    specification_groups = 81,
    source_area_groups = 201,
  },
  classes = {
    accepted_implementation_unestablished = 2460,
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
    {
      requirement_id = "molten.testing.receipt_driven_traceability.stale_receipt_denial",
      implementation_path = "src/testing/traceability/parts/p008/body.rs",
      verification_path = "src/testing/traceability/parts/tests/p001/body.rs",
    },
    {
      requirement_id = "molten.consensus.linearizable_reads",
      implementation_path = "src/fabric_consistency/raft/transition/client.rs",
      verification_path = "src/fabric_consistency/raft/parts/tests/p002/body.rs",
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
