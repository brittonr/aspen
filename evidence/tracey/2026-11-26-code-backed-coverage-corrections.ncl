# Dated correction to the 2026-08-10 runtime-spine direct-repair manifests under code-backed coverage.
# The original manifests are preserved unchanged; the inherited-tracey-debt check reads this record alongside
# them. A withdrawn repair is no longer accepted coverage and must stay in the inherited debt baseline.
# r[impl molten.testing.traceability_code_backed_coverage]
let Withdrawal = {
  manifest | String,
  requirement_id | String,
  reason | String,
}
in
let Supersession = {
  manifest | String,
  requirement_id | String,
  verification_path | String,
  reason | String,
}
in
let ReturnedToDebt = {
  requirement_id | String,
  reason | String,
}
in
let Corrections = {
  schema | String,
  observed_date | String,
  base_revision | String,
  rule | String,
  withdrawn_repairs | Array Withdrawal,
  superseded_verifications | Array Supersession,
  returned_to_debt | Array ReturnedToDebt,
  non_claims | Array String,
}
in
let content_refs = "evidence/tracey/runtime-spine-content-refs-direct-repairs.json" in
let preserves_boundary = "evidence/tracey/runtime-spine-preserves-boundary-direct-repairs.json" in
let non_code = "evidence rests on docs/ Nickel fixtures, prose, flake.nix comments, or archived validation notes, which no longer discharge coverage" in
let validator_deleted = "the only compiled implementation and test, the dead crates/molten-core/src/preserves_profile.rs validator, was deleted during integration; the boundary is now enforced by the Nickel contract docs/preserves-boundary-profile/contracts.ncl, whose markers do not discharge coverage" in
let no_std_gate = "verified only by the molten-core-no-std flake check and scripts/check-molten-core-no-std-sources.sh; no compiled Rust test carries a verify marker, and flake/script markers do not discharge coverage" in
let corrections | Corrections = {
  schema = "molten.tracey-coverage-corrections.v1",
  observed_date = "2026-11-26",
  base_revision = "bfb7a7f4a1d19bc81a11624d019e7c183d5fdadc",
  rule = "coverage requires impl and verify markers in compiled src/, crates/, or tests/ Rust sources reachable from a workspace target",
  withdrawn_repairs = [
    {
      manifest = content_refs,
      requirement_id = "molten.runtime_spine.canonical_content_refs.cleanup_tests",
      reason = "implementation and verification markers exist only as flake.nix shell comments",
    },
    {
      manifest = content_refs,
      requirement_id = "molten.runtime_spine.canonical_content_refs.migration",
      reason = "no compiled test carries a verify marker; the only verification marker is a flake.nix shell comment",
    },
    { manifest = preserves_boundary, requirement_id = "molten.runtime_spine.preserves_boundary_profile.contract", reason = non_code },
    { manifest = preserves_boundary, requirement_id = "molten.runtime_spine.preserves_boundary_profile.docs", reason = non_code },
    { manifest = preserves_boundary, requirement_id = "molten.runtime_spine.preserves_boundary_profile.docs.non_claims", reason = non_code },
    { manifest = preserves_boundary, requirement_id = "molten.runtime_spine.preserves_boundary_profile.final_validation", reason = non_code },
    { manifest = preserves_boundary, requirement_id = "molten.runtime_spine.preserves_boundary_profile.final_validation.fixtures", reason = non_code },
    { manifest = preserves_boundary, requirement_id = "molten.runtime_spine.preserves_boundary_profile.fixtures.negative", reason = non_code },
    { manifest = preserves_boundary, requirement_id = "molten.runtime_spine.preserves_boundary_profile.fixtures.positive", reason = non_code },
    { manifest = preserves_boundary, requirement_id = "molten.runtime_spine.preserves_boundary_profile.validation", reason = validator_deleted },
    { manifest = preserves_boundary, requirement_id = "molten.runtime_spine.preserves_boundary_profile.validation.core_coupling", reason = validator_deleted },
  ],
  superseded_verifications = [],
  returned_to_debt = [
    {
      requirement_id = "molten.project.molten_core_no_std",
      reason = "implementation marker in crates/molten-core/src/lib.rs, but %{no_std_gate}",
    },
    { requirement_id = "molten.testing.molten_core_no_std_ci_gate", reason = no_std_gate },
  ],
  non_claims = [
    "original-manifests-rewritten",
    "withdrawn-requirements-unimplemented",
    "complete-runtime-spine-coverage",
    "release-readiness",
  ],
}
in
corrections
