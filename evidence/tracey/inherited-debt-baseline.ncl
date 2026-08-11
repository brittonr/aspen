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
  baseline_origin_revision = "9842cc6453a00ed943a866549bb4307c8502d575",
  requirement_root = "cairn/specs",
  evidence_roots = ["src", "crates", "tests", "tools", "docs", "scripts", "flake.nix"],
  requirements = 2508,
  referenced = 584,
  uncovered = 1924,
  dangling = 0,
  uncovered_list = "evidence/tracey/inherited-debt-baseline.txt",
  uncovered_list_blake3 = "c67c691d1fb65706aa916d5d906d91a3081d3b2aad947a99a698ac222bef1c19",
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
