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
    blake3 = "68015e0ce09323a3bd878e25d7bccc0ed6eaf316500826528a4671763e1cba4e",
    entries = 2456,
  },
  inventory = {
    path = "evidence/tracey/inherited-debt-classification.tsv",
    blake3 = "5a6338ba9ef40a7ff6d2aa07ade772b3443b41c39edea37314220ec542c2ffba",
    summary_path = "evidence/tracey/inherited-debt-classification.md",
    summary_blake3 = "66509842cd3a63df8f8203ab3806fdf3a3c5f5c48548c32c1b79f36bec45502e",
    entries = 2456,
    specification_groups = 81,
    source_area_groups = 201,
  },
  classes = {
    accepted_implementation_unestablished = 2456,
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
