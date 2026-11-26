# Typed metadata for the inherited Tracey debt baseline.
let InventoryObservation = {
  observed_date | String,
  revision | String,
  requirements | Number,
  uncovered | Number,
  note | String,
}
in
let BaselineMetadata = {
  schema | String,
  observed_date | String,
  baseline_origin_revision | String,
  requirement_root | String,
  coverage_roots | Array String,
  cross_reference_roots | Array String,
  requirements | Number,
  referenced | Number,
  covered | Number,
  uncovered | Number,
  dangling | Number,
  uncompiled_marker_files | Number,
  uncovered_list | String,
  uncovered_list_blake3 | String,
  scanner | String,
  previous_observation | InventoryObservation,
  non_claims | Array String,
}
in
let metadata | BaselineMetadata = {
  schema = "molten.inherited-tracey-debt-baseline.v2",
  observed_date = "2026-11-26",
  baseline_origin_revision = "bfb7a7f4a1d19bc81a11624d019e7c183d5fdadc",
  requirement_root = ".cairn/specs",
  # Only Rust sources reachable from a workspace target's module tree under these roots (through `mod`,
  # `#[path]`, and `include!`, excluding always-false `cfg` items) discharge debt, and only with both `impl` and
  # `verify` markers.
  coverage_roots = ["src", "crates", "tests"],
  # Scanned for dangling identities only; markers here never discharge debt.
  cross_reference_roots = ["tools", "docs", "scripts", "pilots", "nix", "flake.nix"],
  # These counts are the guard's own `requirements=`, `referenced=`, `covered=`, `uncovered=`, `dangling=`, and
  # `uncompiled_marker_files=` output lines; the inherited-tracey-debt check compares each of them.
  # `referenced`: accepted requirements named by any marker (any verb, any scanned root).
  # `covered`: accepted requirements with compiled `impl` and `verify` markers (code-backed coverage).
  requirements = 3147,
  referenced = 1174,
  covered = 686,
  uncovered = 2461,
  dangling = 0,
  uncompiled_marker_files = 0,
  uncovered_list = "evidence/tracey/inherited-debt-baseline.txt",
  uncovered_list_blake3 = "ebaf9c6f57491231ec185c3d8a7fdbc683decf0929ce03fb64e8568323ec2b50",
  scanner = "tools/tracey/inherited_debt_guard.rs",
  # Accepted-inventory change, separate from the coverage-rule change: the 2026-11-13 observation counted 3,057
  # accepted requirements. By base revision bfb7a7f, 78 more accepted requirements had been synced into
  # `.cairn/specs` without refreshing this metadata; the 1,930-entry baseline was unchanged by them. The
  # 2026-11-26 code-backed re-baseline then added 526 entries (requirements whose apparent coverage came from
  # prose, flake comments, uncompiled `tools/tracey/*_proof.rs` stubs, `depends`/`related` markers, or a lone
  # `impl` or `verify` marker) and removed none: 1,930 + 526 = 2,456. Integrating that change with the
  # concurrent swarm changes added two accepted requirements (3,137) and returned four requirements to debt
  # with no removals (2,460); see `evidence/tracey/2026-11-26-code-backed-coverage-corrections.ncl`:
  # `molten.runtime_spine.preserves_boundary_profile.validation{,.core_coupling}` lost their only compiled
  # markers when the dead `crates/molten-core/src/preserves_profile.rs` validator was deleted, and
  # `molten.project.molten_core_no_std` and `molten.testing.molten_core_no_std_ci_gate` are verified only
  # by the `molten-core-no-std` flake check and `scripts/check-molten-core-no-std-sources.sh`. Archiving the
  # code-backed coverage change accepted `molten.testing.traceability_code_backed_coverage` (3,140 requirements
  # after the concurrent merges), and it returned to debt with no removals (2,461): the guard that implements
  # it is implemented and verified in `tools/`, which never discharges coverage. Referenced and covered counts
  # moved with the concurrent CompanionDocs, daemon, and schema de-shard merges. Archiving
  # `repair-companion-doc-source-discrepancies` then accepted seven requirements (3,147), all carrying compiled
  # `impl` and `verify` markers, so covered rose to 686 and the 2,461-entry baseline is unchanged.
  previous_observation = {
    observed_date = "2026-11-13",
    revision = "9842cc6453a00ed943a866549bb4307c8502d575",
    requirements = 3057,
    uncovered = 1930,
    note = "any-marker coverage over src, crates, tests, tools, docs, scripts, pilots, nix, and flake.nix",
  },
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
