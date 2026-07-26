# Pure deterministic validator for Molten's artifact-auth source cutover.
let _digest_hex_chars = 64 in
let _git_oid_hex_chars = 40 in
let _artifact_auth_package_count = 2 in
let _release_package_count = 1 in
let _artifact_auth_policy_rows = 2 in
let _blake3_regex = "^[0-9a-f]{%{std.to_string _digest_hex_chars}}$" in
let _git_oid_regex = "^[0-9a-f]{%{std.to_string _git_oid_hex_chars}}$" in
let _rid_regex = "^rad:z[1-9A-HJ-NP-Za-km-z]+$" in
let _expected = {
  rid = "rad:z4JGYYW7WsesXUq7MXVdx16Fawu2f",
  https_git = "https://git.onix.computer/z4JGYYW7WsesXUq7MXVdx16Fawu2f.git",
  commit = "799459346d5416fbd7b9f55840a7371441b55afa",
  source_archive_blake3 = "246a7cad91e7e8a158e22da21f3bff3e61aa0431a58936b5a739178bc62064c7",
  publication_revision = "e41340bec587b6d049b5cc518ec7db925dde84be",
  publication_blake3 = "e58a3de4d6b3b32a547c3cfe5c3e829292cda73891c7776f214f5d4edce10b1c",
  root_manifest_blake3 = "89cba6df445c212d0014c201b62d26e684575fcf64134f867b0c9034110817df",
  core_manifest_blake3 = "08e87ee69211190a1a181f6c5c44306daa6db89f7b5746a0a365693201da33cd",
  cargo_lock_blake3 = "e3cf729d1bd9cd342a73e496f4474880e1b92e15bfa2c7c3fdc7e531ca142aae",
  flake_blake3 = "92289d8001d76609a48ebf9281a43d1222b15be145190acddf553701c4fa6796",
  flake_lock_blake3 = "7bff28c24abdd3bbee845b57a08ec6e991295d2643f678af4f49d3d06a2dee4c",
  contracts_blake3 = "6ab81958eb173bd3b072b2dc2f118ab8dbbf58a157ae94a114d5ded60a940799",
  profile_blake3 = "4a76dea5427120a6aa44d7dc1e50c64baf7f171ab219b303197600de6312d8e0",
  default_plan_blake3 = "93a69fb90f7f7c92fa5ed261d76d81a0e7a245c9aadb7797e5d73ea97219f651",
  release_plan_blake3 = "5ba77071bb29b8150049565a31fa6f91bdfdfdad7cf41f711e9359b233aacb1d",
  default_inputs_sha256 = "58292c3843bdf2dd94a9f8bbb741156d855cd7a6aea2537ea51df359f87c07f3",
  release_inputs_sha256 = "5c119b6c7eb939326055c524a6e928fb9e29047f3d4c385a1a885b60b1730770",
  nar_hash = "sha256-nEgz2FtVuDesX95yyxidp0vhjxL4INB6Ve8rkpLyJk0=",
  packages = ["artifact-auth-core", "artifact-auth-ed25519"],
  release_packages = ["artifact-auth-core"],
}
in
let _expected_lock_source = "git+%{_expected.https_git}?rev=%{_expected.commit}#%{_expected.commit}" in
let _required_root_fields = [
  "schema_version",
  "receipt_type",
  "status",
  "observed_date",
  "source",
  "publication",
  "cargo",
  "nix",
  "release_policy",
  "unit2nix",
  "validation",
  "fallback",
  "rollback",
  "authority_boundary",
  "non_claims",
  "evidence",
]
in
let _required_non_claims = [
  "artifact-auth-source-correctness",
  "whole-molten-behavior-correctness",
  "forge-availability",
  "automatic-fallback",
  "standalone-authority-admission",
  "runtime-or-release-authority",
  "whole-stack-github-independence",
  "release-readiness",
]
in
let _has_fields = fun fields value =>
  std.is_record value && fields |> std.array.all (fun field => std.record.has_field field value)
in
let _is_blake3 = fun value => std.is_string value && std.string.is_match _blake3_regex value in
let _is_git_oid = fun value => std.is_string value && std.string.is_match _git_oid_regex value in
let _is_rid = fun value => std.is_string value && std.string.is_match _rid_regex value in
let _error = fun error_code error_message => { code = error_code, message = error_message } in
let _first_error = fun errors =>
  errors
  |> std.array.filter_map (fun error => if error == null then 'None else 'Some error)
  |> match { [] => 'Ok, [error, ..] => 'Error error }
in
let _validate = fun receipt =>
  if !_has_fields _required_root_fields receipt then
    'Error (_error "cutover-shape" "cutover evidence is incomplete")
  else
    _first_error [
      if receipt.schema_version == 1
      && receipt.receipt_type == "molten.artifact-auth-radicle-cutover.v1"
      && receipt.status == "accepted"
      && receipt.observed_date == "2026-07-25" then
        null
      else
        _error "cutover-header" "cutover identity or status drifted",
      if _has_fields ["rid", "https_git", "reviewed_commit", "source_archive_blake3"] receipt.source
      && _is_rid receipt.source.rid
      && receipt.source.rid == _expected.rid
      && receipt.source.https_git == _expected.https_git
      && _is_git_oid receipt.source.reviewed_commit
      && receipt.source.reviewed_commit == _expected.commit
      && _is_blake3 receipt.source.source_archive_blake3
      && receipt.source.source_archive_blake3 == _expected.source_archive_blake3 then
        null
      else
        _error "cutover-source" "RID, HTTPS URL, commit, or source archive identity drifted",
      if receipt.publication.revision == _expected.publication_revision
      && receipt.publication.receipt_type == "artifact-auth.radicle-publication.v1"
      && _is_blake3 receipt.publication.receipt_blake3
      && receipt.publication.receipt_blake3 == _expected.publication_blake3
      && receipt.publication.status == "accepted" then
        null
      else
        _error "cutover-publication" "accepted publication linkage drifted",
      if receipt.cargo.root_manifest_blake3 == _expected.root_manifest_blake3
      && receipt.cargo.core_manifest_blake3 == _expected.core_manifest_blake3
      && receipt.cargo.lock_blake3 == _expected.cargo_lock_blake3
      && receipt.cargo.lock_source == _expected_lock_source
      && std.array.length receipt.cargo.packages == _artifact_auth_package_count
      && receipt.cargo.packages == _expected.packages
      && receipt.cargo.package_version == "0.1.0"
      && !receipt.cargo.crate_graph_changed then
        null
      else
        _error "cutover-cargo" "Cargo source, packages, lock identity, or graph drifted",
      if receipt.nix.flake_blake3 == _expected.flake_blake3
      && receipt.nix.lock_blake3 == _expected.flake_lock_blake3
      && receipt.nix.locked_url == _expected.https_git
      && receipt.nix.original_url == _expected.https_git
      && receipt.nix.locked_rev == _expected.commit
      && receipt.nix.original_rev == _expected.commit
      && receipt.nix.nar_hash == _expected.nar_hash
      && receipt.nix.nar_hash_preserved then
        null
      else
        _error "cutover-nix" "Nix source, revision, narHash, or lock identity drifted",
      if receipt.release_policy.contracts_blake3 == _expected.contracts_blake3
      && receipt.release_policy.profile_blake3 == _expected.profile_blake3
      && receipt.release_policy.artifact_auth_rows == _artifact_auth_policy_rows
      && receipt.release_policy.source_coordinate == _expected.https_git
      && receipt.release_policy.immutable_revision == _expected.commit
      && receipt.release_policy.transport_policy == "https" then
        null
      else
        _error "cutover-policy" "release dependency policy drifted",
      if receipt.unit2nix.default_plan_blake3 == _expected.default_plan_blake3
      && receipt.unit2nix.release_plan_blake3 == _expected.release_plan_blake3
      && receipt.unit2nix.default_inputs_sha256 == _expected.default_inputs_sha256
      && receipt.unit2nix.release_inputs_sha256 == _expected.release_inputs_sha256
      && std.array.length receipt.unit2nix.default_packages == _artifact_auth_package_count
      && receipt.unit2nix.default_packages == _expected.packages
      && std.array.length receipt.unit2nix.release_packages == _release_package_count
      && receipt.unit2nix.release_packages == _expected.release_packages
      && receipt.unit2nix.generated_by == "unit2nix-d4883180de0ce3033b7e4e2ab4216f33134863c5" then
        null
      else
        _error "cutover-unit2nix" "generated build-plan source or identity drifted",
      if receipt.validation.baseline_revision == "f9ad1c0c16c7bb3c67dcb0d87f5910f287679946"
      && receipt.validation.baseline_core == "passed-10"
      && receipt.validation.baseline_shell == "passed-10"
      && receipt.validation.post_cutover_core == "passed-10"
      && receipt.validation.post_cutover_shell == "passed-10"
      && receipt.validation.formatting == "passed"
      && receipt.validation.focused_clippy == "passed"
      && receipt.validation.source_agreement_positive == "passed"
      && receipt.validation.source_agreement_negative == "passed"
      && !receipt.validation.rust_implementation_changed then
        null
      else
        _error "cutover-validation" "behavioral or source-agreement evidence is incomplete",
      if !receipt.fallback.manifest_github_source
      && !receipt.fallback.cargo_lock_github_source
      && !receipt.fallback.nix_input_github_source
      && !receipt.fallback.nix_lock_github_source
      && !receipt.fallback.release_policy_github_source
      && !receipt.fallback.default_plan_github_source
      && !receipt.fallback.release_plan_github_source
      && !receipt.fallback.automatic_fallback
      && receipt.fallback.outage_result == "visible-resolution-failure" then
        null
      else
        _error "cutover-fallback" "an executable GitHub or automatic fallback remains",
      if !receipt.rollback.automatic
      && receipt.rollback.requires_owning_tool_regeneration
      && receipt.rollback.requires_source_identity_review then
        null
      else
        _error "cutover-rollback" "rollback bypasses regeneration or identity review",
      if receipt.authority_boundary.legacy_authoritative
      && !receipt.authority_boundary.standalone_authority_admitted
      && receipt.authority_boundary.product_authority_retained then
        null
      else
        _error "cutover-authority" "transport cutover widened standalone or product authority",
      if _required_non_claims |> std.array.all (fun claim => std.array.elem claim receipt.non_claims) then
        null
      else
        _error "cutover-non-claims" "required non-claims are missing",
      if receipt.evidence |> std.array.all (fun path => std.is_string path && !(std.string.is_match "^/" path)) then
        null
      else
        _error "cutover-evidence" "evidence paths must remain relative",
      if !(std.record.has_field "claims" receipt)
      && !(std.record.has_field "credentials" receipt)
      && !(std.record.has_field "fallback_url" receipt) then
        null
      else
        _error "cutover-overclaim" "receipt contains authority, credentials, or fallback",
    ]
in
{
  expected = _expected,
  expected_lock_source = _expected_lock_source,
  required_non_claims = _required_non_claims,
  is_blake3 = _is_blake3,
  is_git_oid = _is_git_oid,
  is_rid = _is_rid,
  validate = _validate,
}
