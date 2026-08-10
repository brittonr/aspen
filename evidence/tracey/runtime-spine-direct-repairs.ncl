# Typed direct-evidence manifest for the runtime-spine Tracey batch.
# r[impl molten.project.runtime_spine_tracey.direct_repairs]
# r[impl molten.project.runtime_spine_tracey.exact_manifest]
# r[impl molten.project.runtime_spine_tracey.non_claims]
let Repair = {
  requirement_id | String,
  source_area | String,
  implementation_path | String,
  verification_path | String,
  evidence_scope | String,
}
in
let Manifest = {
  schema | String,
  observed_date | String,
  base_revision | String,
  accepted_specification | String,
  baseline_before | Number,
  baseline_after | Number,
  runtime_spine_before | Number,
  runtime_spine_after | Number,
  repairs | Array Repair,
  non_claims | Array String,
}
in
let manifest | Manifest = {
  schema = "molten.runtime-spine-tracey-direct-repairs.v1",
  observed_date = "2026-08-10",
  base_revision = "4dd9081da8137c10abcd2fac225d982db1d5d0a8",
  accepted_specification = "cairn/specs/runtime-spine/spec.md",
  baseline_before = 1979,
  baseline_after = 1965,
  runtime_spine_before = 442,
  runtime_spine_after = 428,
  repairs = [
    {
      requirement_id = "molten.preserves_boundary_field_contracts.field_contract_denials",
      source_area = "preserves_boundary_field_contracts",
      implementation_path = "src/preserves/parts/rail/p001/body.rs",
      verification_path = "src/preserves/parts/rail/p001/body.rs",
      evidence_scope = "negative field-domain and duplicate-value validation",
    },
    {
      requirement_id = "molten.preserves_boundary_field_contracts.field_contracts",
      source_area = "preserves_boundary_field_contracts",
      implementation_path = "src/preserves/parts/rail/p001/body.rs",
      verification_path = "src/preserves/parts/rail/p001/body.rs",
      evidence_scope = "typed boundary field validation",
    },
    {
      requirement_id = "molten.preserves_canonical_bytes.noncanonical_denial",
      source_area = "preserves_canonical_bytes",
      implementation_path = "src/preserves/parts/rail/p001/body.rs",
      verification_path = "src/preserves/parts/rail/p001/body.rs",
      evidence_scope = "noncanonical and tampered packed-byte denial",
    },
    {
      requirement_id = "molten.preserves_canonical_bytes.strict_decode",
      source_area = "preserves_canonical_bytes",
      implementation_path = "src/preserves/parts/rail/p001/body.rs",
      verification_path = "src/preserves/parts/rail/p001/body.rs",
      evidence_scope = "canonical packed-byte decode and identity",
    },
    {
      requirement_id = "molten.preserves_rail_toolkit.check_sets",
      source_area = "preserves_rail_toolkit",
      implementation_path = "src/preserves/parts/rail/p001/body.rs",
      verification_path = "src/preserves/parts/rail/p001/body.rs",
      evidence_scope = "shared checks builder, parser, and required-check denial",
    },
    {
      requirement_id = "molten.preserves_rail_toolkit.negative_shapes",
      source_area = "preserves_rail_toolkit",
      implementation_path = "src/preserves/parts/rail/p001/body.rs",
      verification_path = "src/preserves/parts/rail/p001/body.rs",
      evidence_scope = "wrong label, arity, type, ref, and check-shape denial",
    },
    {
      requirement_id = "molten.preserves_rail_toolkit.parser_builders",
      source_area = "preserves_rail_toolkit",
      implementation_path = "src/preserves/parts/rail/p001/body.rs",
      verification_path = "src/preserves/parts/rail/p001/body.rs",
      evidence_scope = "shared simple-record, string, ref, optional-ref, and sequence helpers",
    },
    {
      requirement_id = "molten.preserves_schema_boundaries.schema_adapter",
      source_area = "preserves_schema_boundaries",
      implementation_path = "src/preserves/parts/rail/p001/body.rs",
      verification_path = "src/preserves/parts/rail/p001/body.rs",
      evidence_scope = "pure deterministic schema validation",
    },
    {
      requirement_id = "molten.preserves_schema_boundaries.schema_artifacts",
      source_area = "preserves_schema_boundaries",
      implementation_path = "src/preserves/parts/rail/p001/body.rs",
      verification_path = "src/preserves/parts/rail/p001/body.rs",
      evidence_scope = "versioned schema artifact and canonical schema ref",
    },
    {
      requirement_id = "molten.preserves_schema_boundaries.schema_denials",
      source_area = "preserves_schema_boundaries",
      implementation_path = "src/preserves/parts/rail/p001/body.rs",
      verification_path = "src/preserves/parts/rail/p001/body.rs",
      evidence_scope = "wrong label, field, type, checks, version, and ref denial",
    },
    {
      requirement_id = "molten.preserves_value_inspection.ambient_token_denial",
      source_area = "preserves_value_inspection",
      implementation_path = "src/preserves/parts/rail/p001/body.rs",
      verification_path = "src/job/parts/dag/tests/m000/p004/body.rs",
      evidence_scope = "structural ambient job-token denial before job admission",
    },
    {
      requirement_id = "molten.preserves_value_inspection.marker_detection",
      source_area = "preserves_value_inspection",
      implementation_path = "src/preserves/parts/rail/p001/body.rs",
      verification_path = "src/preserves/parts/rail/p001/body.rs",
      evidence_scope = "nested sensitive record and symbol detection",
    },
    {
      requirement_id = "molten.preserves_value_inspection.ref_retention",
      source_area = "preserves_value_inspection",
      implementation_path = "src/preserves/parts/rail/p001/body.rs",
      verification_path = "src/preserves/parts/rail/p001/body.rs",
      evidence_scope = "nested structural content-ref lookup",
    },
    {
      requirement_id = "molten.preserves_value_inspection.structural_scan",
      source_area = "preserves_value_inspection",
      implementation_path = "src/preserves/parts/rail/p001/body.rs",
      verification_path = "src/preserves/parts/rail/p001/body.rs",
      evidence_scope = "bounded structural traversal without rendered-text inference",
    },
  ],
  non_claims = [
    "complete-runtime-spine-coverage",
    "all-preserves-trust-boundaries-use-strict-decode",
    "behavioral-correctness-beyond-listed-tests",
    "release-readiness",
    "whole-system-correctness",
  ],
}
in
manifest
