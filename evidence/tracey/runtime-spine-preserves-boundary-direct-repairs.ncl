# Typed direct-evidence manifest for the runtime-spine Preserves boundary-profile batch.
# r[impl molten.project.runtime_spine_preserves_boundary_tracey.direct_repairs]
# r[impl molten.project.runtime_spine_preserves_boundary_tracey.exact_manifest]
# r[impl molten.project.runtime_spine_preserves_boundary_tracey.non_claims]
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
  candidate_count | Number,
  repairs | Array Repair,
  rejected_candidates | Array String,
  non_claims | Array String,
}
in
let manifest | Manifest = {
  schema = "molten.runtime-spine-preserves-boundary-tracey-direct-repairs.v1",
  observed_date = "2026-08-10",
  base_revision = "9842cc6453a00ed943a866549bb4307c8502d575",
  accepted_specification = "cairn/specs/runtime-spine/spec.md",
  baseline_before = 1933,
  baseline_after = 1924,
  runtime_spine_before = 396,
  runtime_spine_after = 387,
  candidate_count = 9,
  repairs = [
    {
      requirement_id = "molten.runtime_spine.preserves_boundary_profile.contract",
      source_area = "runtime_spine",
      implementation_path = "docs/preserves-boundary-profile/contracts.ncl",
      verification_path = "docs/preserves-boundary-profile/valid.ncl",
      evidence_scope = "typed rows record all required ownership, identity, consumer, DTO, canonical-byte, and non-claim fields for five admitted families",
    },
    {
      requirement_id = "molten.runtime_spine.preserves_boundary_profile.docs",
      source_area = "runtime_spine",
      implementation_path = "docs/modularity-boundaries.md",
      verification_path = "flake.nix",
      evidence_scope = "the modularity guide bounds profile success to canonical boundary identity and adapter placement",
    },
    {
      requirement_id = "molten.runtime_spine.preserves_boundary_profile.docs.non_claims",
      source_area = "runtime_spine",
      implementation_path = "docs/modularity-boundaries.md",
      verification_path = "flake.nix",
      evidence_scope = "the guide explicitly denies transport liveness, actor authority correctness, replay completeness, and Valence Evidence IR acceptance",
    },
    {
      requirement_id = "molten.runtime_spine.preserves_boundary_profile.final_validation",
      source_area = "runtime_spine",
      implementation_path = "flake.nix",
      verification_path = "cairn/archive/1970-01-01-repair-runtime-spine-preserves-boundary-tracey/evidence/validation.md",
      evidence_scope = "focused Rust and Nickel checks plus formatting, Tiger Style, Octet, Cairn, and full Nix validation run before archive",
    },
    {
      requirement_id = "molten.runtime_spine.preserves_boundary_profile.final_validation.fixtures",
      source_area = "runtime_spine",
      implementation_path = "flake.nix",
      verification_path = "cairn/archive/1970-01-01-repair-runtime-spine-preserves-boundary-tracey/evidence/validation.md",
      evidence_scope = "repository validation exports the valid profile and requires every invalid fixture to fail",
    },
    {
      requirement_id = "molten.runtime_spine.preserves_boundary_profile.fixtures.negative",
      source_area = "runtime_spine",
      implementation_path = "docs/preserves-boundary-profile/non-canonical.ncl",
      verification_path = "flake.nix",
      evidence_scope = "Nickel and Rust negatives cover non-canonical bytes, missing labels, stale refs, unsupported consumers, missing non-claims, and raw core coupling",
    },
    {
      requirement_id = "molten.runtime_spine.preserves_boundary_profile.fixtures.positive",
      source_area = "runtime_spine",
      implementation_path = "docs/preserves-boundary-profile/valid.ncl",
      verification_path = "flake.nix",
      evidence_scope = "the valid fixture covers canonical node-control, ticket, workflow-bundle, receipt, and evidence-envelope measurements",
    },
    {
      requirement_id = "molten.runtime_spine.preserves_boundary_profile.validation",
      source_area = "runtime_spine",
      implementation_path = "crates/molten-core/src/preserves_profile.rs",
      verification_path = "crates/molten-core/src/preserves_profile.rs",
      evidence_scope = "the pure deterministic core validates typed measurements and produces explicit issue values without IO",
    },
    {
      requirement_id = "molten.runtime_spine.preserves_boundary_profile.validation.core_coupling",
      source_area = "runtime_spine",
      implementation_path = "crates/molten-core/src/preserves_profile.rs",
      verification_path = "docs/preserves-boundary-profile/raw-core-coupling.ncl",
      evidence_scope = "adapter-only rows reject supplied raw-Preserves core-coupling facts and molten-core has no Preserves dependency or import",
    },
  ],
  rejected_candidates = [],
  non_claims = [
    "transport-liveness",
    "actor-authority-correctness",
    "replay-completeness",
    "valence-evidence-ir-acceptance",
    "universal-adapter-separation",
    "complete-runtime-spine-coverage",
    "release-readiness",
    "whole-system-correctness",
  ],
}
in
manifest
