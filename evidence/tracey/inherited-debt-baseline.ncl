# Typed metadata for the inherited Tracey debt baseline.
let BaselineMetadata = {
  schema | String,
  observed_date | String,
  baseline_origin_revision | String,
  requirement_root | String,
  evidence_roots | Array String,
  requirements | Number,
  referenced | Number,
  uncovered | Number,
  dangling | Number,
  uncovered_list | String,
  uncovered_list_blake3 | String,
  scanner | String,
  non_claims | Array String,
}
in
let metadata | BaselineMetadata = {
  schema = "molten.inherited-tracey-debt-baseline.v1",
  observed_date = "2026-11-13",
  baseline_origin_revision = "9842cc6453a00ed943a866549bb4307c8502d575",
  requirement_root = ".cairn/specs",
  evidence_roots = ["src", "crates", "tests", "tools", "docs", "scripts", "pilots", "nix", "flake.nix"],
  requirements = 3057,
  referenced = 1125,
  uncovered = 1932,
  dangling = 0,
  uncovered_list = "evidence/tracey/inherited-debt-baseline.txt",
  uncovered_list_blake3 = "7b7eb39ded640771e4d74a79a4e1c0856f5eb0c609248538491a09b6070db93f",
  scanner = "tools/tracey/inherited_debt_guard.rs",
  non_claims = [
    "uncovered-requirement-exemption",
    "marker-truth",
    "behavioral-correctness",
    "release-readiness",
    "whole-system-correctness",
  ],
}
in
metadata
