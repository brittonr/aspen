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
  baseline_origin_revision = "e275a9520795b81c797f5013357d48d5d419477b",
  requirement_root = "cairn/specs",
  evidence_roots = ["src", "crates", "tests", "tools", "docs", "scripts", "flake.nix"],
  requirements = 2504,
  referenced = 571,
  uncovered = 1933,
  dangling = 0,
  uncovered_list = "evidence/tracey/inherited-debt-baseline.txt",
  uncovered_list_blake3 = "9e0ffae8e1c4727c33e16f34ac87d94b8ee54e2312c20b02a00429475d8170a0",
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
