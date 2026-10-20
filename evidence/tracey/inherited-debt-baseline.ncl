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
  observed_date = "2026-10-19",
  baseline_origin_revision = "9842cc6453a00ed943a866549bb4307c8502d575",
  requirement_root = ".cairn/specs",
  evidence_roots = ["src", "crates", "tests", "tools", "docs", "scripts", "flake.nix"],
  requirements = 2834,
  referenced = 901,
  uncovered = 1933,
  dangling = 0,
  uncovered_list = "evidence/tracey/inherited-debt-baseline.txt",
  uncovered_list_blake3 = "e32ca33a172ba31f3f81c5ca26ffbfab225bc24ec3c37f8760dd95e9bab92bc1",
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
