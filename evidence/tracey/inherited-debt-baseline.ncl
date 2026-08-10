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
  baseline_origin_revision = "264294ba091b2e9d1c777bc6728cc64a1ce876c7",
  requirement_root = "cairn/specs",
  evidence_roots = ["src", "crates", "tests", "tools", "docs", "scripts", "flake.nix"],
  requirements = 2488,
  referenced = 509,
  uncovered = 1979,
  dangling = 0,
  uncovered_list = "evidence/tracey/inherited-debt-baseline.txt",
  uncovered_list_blake3 = "334360f765acce463b56573883443a93a0b6764c01e82b36b90594c31d854ff0",
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
