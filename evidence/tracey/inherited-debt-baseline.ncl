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
  observed_date = "2026-08-10",
  baseline_origin_revision = "9bf44787f49a906f2da0541a5790492eb609aaab",
  requirement_root = "cairn/specs",
  evidence_roots = ["src", "crates", "tests", "tools", "docs", "scripts", "flake.nix"],
  requirements = 2480,
  referenced = 499,
  uncovered = 1981,
  dangling = 0,
  uncovered_list = "evidence/tracey/inherited-debt-baseline.txt",
  uncovered_list_blake3 = "1653e3284ab2e9f13730c87ced9cc7ab04763dfbd7cdf4cc6511a4bfa246f6b8",
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
