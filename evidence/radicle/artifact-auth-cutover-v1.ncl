# Typed evidence for Molten's artifact-auth transport cutover.
let CutoverReceipt = {
  schema_version | Number,
  receipt_type | String,
  status | String,
  observed_date | String,
  source
    | {
      rid | String,
      https_git | String,
      reviewed_commit | String,
      source_archive_blake3 | String,
    },
  publication
    | {
      revision | String,
      receipt_type | String,
      receipt_blake3 | String,
      status | String,
    },
  cargo
    | {
      root_manifest_blake3 | String,
      core_manifest_blake3 | String,
      lock_blake3 | String,
      lock_source | String,
      packages | Array String,
      package_version | String,
      crate_graph_changed | Bool,
    },
  nix
    | {
      flake_blake3 | String,
      lock_blake3 | String,
      locked_url | String,
      original_url | String,
      locked_rev | String,
      original_rev | String,
      nar_hash | String,
      nar_hash_preserved | Bool,
    },
  release_policy
    | {
      contracts_blake3 | String,
      profile_blake3 | String,
      artifact_auth_rows | Number,
      source_coordinate | String,
      immutable_revision | String,
      transport_policy | String,
    },
  unit2nix
    | {
      default_plan_blake3 | String,
      release_plan_blake3 | String,
      default_inputs_sha256 | String,
      release_inputs_sha256 | String,
      default_packages | Array String,
      release_packages | Array String,
      generated_by | String,
    },
  validation
    | {
      baseline_revision | String,
      baseline_core | String,
      baseline_shell | String,
      post_cutover_core | String,
      post_cutover_shell | String,
      formatting | String,
      focused_clippy | String,
      source_agreement_positive | String,
      source_agreement_negative | String,
      rust_implementation_changed | Bool,
    },
  fallback
    | {
      manifest_github_source | Bool,
      cargo_lock_github_source | Bool,
      nix_input_github_source | Bool,
      nix_lock_github_source | Bool,
      release_policy_github_source | Bool,
      default_plan_github_source | Bool,
      release_plan_github_source | Bool,
      automatic_fallback | Bool,
      outage_result | String,
    },
  rollback
    | {
      automatic | Bool,
      requires_owning_tool_regeneration | Bool,
      requires_source_identity_review | Bool,
    },
  authority_boundary
    | {
      legacy_authoritative | Bool,
      standalone_authority_admitted | Bool,
      product_authority_retained | Bool,
    },
  non_claims | Array String,
  evidence | Array String,
}
in
let expected_reviewed_commit = "799459346d5416fbd7b9f55840a7371441b55afa" in
let expected_https_git = "https://git.onix.computer/z4JGYYW7WsesXUq7MXVdx16Fawu2f.git" in
let expected_lock_source = "git+%{expected_https_git}?rev=%{expected_reviewed_commit}#%{expected_reviewed_commit}" in
let receipt | CutoverReceipt = {
  schema_version = 1,
  receipt_type = "molten.artifact-auth-radicle-cutover.v1",
  status = "accepted",
  observed_date = "2026-07-25",
  source = {
    rid = "rad:z4JGYYW7WsesXUq7MXVdx16Fawu2f",
    https_git = expected_https_git,
    reviewed_commit = expected_reviewed_commit,
    source_archive_blake3 = "246a7cad91e7e8a158e22da21f3bff3e61aa0431a58936b5a739178bc62064c7",
  },
  publication = {
    revision = "e41340bec587b6d049b5cc518ec7db925dde84be",
    receipt_type = "artifact-auth.radicle-publication.v1",
    receipt_blake3 = "e58a3de4d6b3b32a547c3cfe5c3e829292cda73891c7776f214f5d4edce10b1c",
    status = "accepted",
  },
  cargo = {
    root_manifest_blake3 = "805ac41b349d4be072c286cfb0697aab69071e67abe1b213a6650aa4383c4d64",
    core_manifest_blake3 = "a7b7ed3e4c313fa88d826ebefe50622eb3984d3664272c896543c3c57938dc36",
    lock_blake3 = "70fb6959d9c2ebbc696979c65abed6df2e4489779dfe6aa88c233adb9f415581",
    lock_source = expected_lock_source,
    packages = ["artifact-auth-core", "artifact-auth-ed25519"],
    package_version = "0.1.0",
    crate_graph_changed = false,
  },
  nix = {
    flake_blake3 = "66da14a7685cee33e1186274f6fa2209a2b03b01844cc264fe0940569855fb93",
    lock_blake3 = "5fa83c35f3c8c5bc12452273962b7dd54e79dbf60f4c390ce61f94cb6281ee29",
    locked_url = expected_https_git,
    original_url = expected_https_git,
    locked_rev = expected_reviewed_commit,
    original_rev = expected_reviewed_commit,
    nar_hash = "sha256-nEgz2FtVuDesX95yyxidp0vhjxL4INB6Ve8rkpLyJk0=",
    nar_hash_preserved = true,
  },
  release_policy = {
    contracts_blake3 = "6ab81958eb173bd3b072b2dc2f118ab8dbbf58a157ae94a114d5ded60a940799",
    profile_blake3 = "c2ed65907748cea694cfba9e5193944785b1330d88964c4274674d21eafc95ee",
    artifact_auth_rows = 2,
    source_coordinate = expected_https_git,
    immutable_revision = expected_reviewed_commit,
    transport_policy = "https",
  },
  unit2nix = {
    default_plan_blake3 = "0b3d282b83c24567ecf61ca73d2833ac51ae168bb528d6c38c4bfac5177af766",
    release_plan_blake3 = "ed207255a1ea0951ca4268b2f5892967a9b35a7dd1a94a10f0f4a88edb3ff2a8",
    default_inputs_sha256 = "36b254817c0e250a9878e8377d087275a332c9c9a64ef50f6692d044bf66af44",
    release_inputs_sha256 = "ed3327aea4079841707e111f2e4a794466aff30da754567bec0bec354578c055",
    default_packages = ["artifact-auth-core", "artifact-auth-ed25519"],
    release_packages = ["artifact-auth-core"],
    generated_by = "unit2nix-d4883180de0ce3033b7e4e2ab4216f33134863c5",
  },
  validation = {
    baseline_revision = "f9ad1c0c16c7bb3c67dcb0d87f5910f287679946",
    baseline_core = "passed-10",
    baseline_shell = "passed-10",
    post_cutover_core = "passed-10",
    post_cutover_shell = "passed-10",
    formatting = "passed",
    focused_clippy = "passed",
    source_agreement_positive = "passed",
    source_agreement_negative = "passed",
    rust_implementation_changed = false,
  },
  fallback = {
    manifest_github_source = false,
    cargo_lock_github_source = false,
    nix_input_github_source = false,
    nix_lock_github_source = false,
    release_policy_github_source = false,
    default_plan_github_source = false,
    release_plan_github_source = false,
    automatic_fallback = false,
    outage_result = "visible-resolution-failure",
  },
  rollback = {
    automatic = false,
    requires_owning_tool_regeneration = true,
    requires_source_identity_review = true,
  },
  authority_boundary = {
    legacy_authoritative = true,
    standalone_authority_admitted = false,
    product_authority_retained = true,
  },
  non_claims = [
    "artifact-auth-source-correctness",
    "whole-molten-behavior-correctness",
    "forge-availability",
    "automatic-fallback",
    "standalone-authority-admission",
    "runtime-or-release-authority",
    "whole-stack-github-independence",
    "release-readiness",
  ],
  evidence = [
    "cairn/archive/2026-07-25-consume-artifact-auth-from-radicle/evidence/source-cutover-2026-07-25.md",
    "evidence/radicle/artifact-auth-cutover-v1.json",
    "artifact-auth:evidence/radicle/publication-v1.json",
    "onix-core:evidence/radicle/source-admission-v1.json",
    "valence:evidence/radicle/artifact-auth-cutover-v1.json",
  ],
}
in
receipt
