# Pure validator for Molten's unified Artifact source migration.
let DigestHexChars = 64 in
let GitOidHexChars = 40 in
let ConsumerPackageCount = 3 in
let SourcePackageCount = 4 in
let Blake3Regex = "^[0-9a-f]{%{std.to_string DigestHexChars}}$" in
let GitOidRegex = "^[0-9a-f]{%{std.to_string GitOidHexChars}}$" in
let Expected = {
  repository = "ssh://git@github.com/OnixResearch/onix-artifact.git",
  profile_repository = "https://github.com/OnixResearch/onix-artifact",
  revision = "c932138d880ddf4c2967f4c024b489b5c0022bf1",
  nar_hash = "sha256-XGQLG60DNeY9FUYcOmn6cfYnhCIJzyqf+VW9yofDYFU=",
  source_archive_blake3 = "3878cdb892bfd4a8eac5779023ba32871d61bc3e8ec7c9ef5f7ab790a8acfeb9",
  license = "MIT OR Apache-2.0",
  source_packages = ["artifact-auth-core", "artifact-auth-ed25519", "artifact-binding-core", "artifact-transfer-core"],
  consumer_packages = ["artifact-auth-core", "artifact-auth-ed25519", "artifact-binding-core"],
  core_blake3 = "839ea3e35429e2a1252020dd779b46ebd185b6911674036820da118d487d1d21",
  ed25519_blake3 = "7fd3987fe3ddc6d30efca37878952135f19905a00b52f349db237054be6b3633",
  historical_repository = "https://git.onix.computer/z4JGYYW7WsesXUq7MXVdx16Fawu2f.git",
  historical_revision = "799459346d5416fbd7b9f55840a7371441b55afa",
  historical_receipt_blake3 = "a71619befdcef0c09fef0e45b82b87b3e786864d50471e0ca43c806b2e3aa1b4",
  baseline_revision = "d69d4d734886ac40dada69cb0bda27985b0cd1de",
}
in
let ExpectedLockSource = "git+%{Expected.repository}?rev=%{Expected.revision}#%{Expected.revision}" in
let RequiredFields = ["schema_version", "receipt_type", "status", "observed_date", "source", "consumer", "parity", "validation", "history", "rollback", "non_claims", "evidence"] in
let RequiredNonClaims = ["artifact-source-correctness", "whole-molten-behavior-correctness", "forge-availability", "automatic-fallback", "runtime-or-release-authority", "release-readiness"] in
let HasFields = fun fields value => std.is_record value && fields |> std.array.all (fun field => std.record.has_field field value) in
let IsBlake3 = fun value => std.is_string value && std.string.is_match Blake3Regex value in
let IsGitOid = fun value => std.is_string value && std.string.is_match GitOidRegex value in
let Error = fun error_code error_message => { code = error_code, message = error_message } in
let FirstError = fun errors =>
  errors
  |> std.array.filter_map (fun candidate => if candidate == null then 'None else 'Some candidate)
  |> match { [] => 'Ok, [candidate, ..] => 'Error candidate }
in
let Validate = fun receipt =>
  if !HasFields RequiredFields receipt then
    'Error (Error "migration-shape" "migration evidence is incomplete")
  else
    FirstError [
      if receipt.schema_version == 1 && receipt.receipt_type == "molten.artifact-source-migration.v1" && receipt.status == "accepted" && receipt.observed_date == "2026-08-06" then null else Error "migration-header" "migration identity or status drifted",
      if receipt.source.repository == Expected.repository && receipt.source.profile_repository == Expected.profile_repository && IsGitOid receipt.source.revision && receipt.source.revision == Expected.revision && receipt.source.nar_hash == Expected.nar_hash && IsBlake3 receipt.source.source_archive_blake3 && receipt.source.source_archive_blake3 == Expected.source_archive_blake3 && receipt.source.license == Expected.license && std.array.length receipt.source.packages == SourcePackageCount && receipt.source.packages == Expected.source_packages then null else Error "migration-source" "unified source identity, package set, or license drifted",
      if receipt.consumer.cargo_lock_source == ExpectedLockSource && std.array.length receipt.consumer.packages == ConsumerPackageCount && receipt.consumer.packages == Expected.consumer_packages && !receipt.consumer.transfer_package_resolved && receipt.consumer.binding_package_resolved && receipt.consumer.cargo_nix_policy_plans_aligned && receipt.consumer.single_nix_input then null else Error "migration-consumer" "Molten consumer graph or generated source agreement drifted",
      if receipt.parity.core_blake3 == Expected.core_blake3 && receipt.parity.ed25519_blake3 == Expected.ed25519_blake3 && !receipt.parity.rust_implementation_changed then null else Error "migration-parity" "authentication source identity or implementation boundary drifted",
      if receipt.validation.baseline_revision == Expected.baseline_revision && receipt.validation.baseline_core == "passed-4" && receipt.validation.baseline_source_check == "passed" && receipt.validation.baseline_cairn == "blocked-policy-schema" && receipt.validation.post_core == "passed" && receipt.validation.post_binding == "passed" && receipt.validation.formatting == "passed" && receipt.validation.clippy == "passed" && receipt.validation.source_admission_positive == "passed" && receipt.validation.source_admission_negative == "passed" then null else Error "migration-validation" "positive or negative validation evidence is incomplete",
      if receipt.history.repository == Expected.historical_repository && receipt.history.revision == Expected.historical_revision && receipt.history.receipt_blake3 == Expected.historical_receipt_blake3 && receipt.history.status == "historical-only" then null else Error "migration-history" "historical source boundary drifted",
      if !receipt.rollback.automatic && receipt.rollback.requires_owning_tool_regeneration && receipt.rollback.requires_source_identity_review then null else Error "migration-rollback" "rollback bypasses regeneration or source review",
      if RequiredNonClaims |> std.array.all (fun claim => std.array.elem claim receipt.non_claims) then null else Error "migration-non-claims" "required non-claims are missing",
      if receipt.evidence |> std.array.all (fun path => std.is_string path && !(std.string.is_match "^/" path)) then null else Error "migration-evidence" "evidence paths must remain relative",
      if !(std.record.has_field "claims" receipt) && !(std.record.has_field "credentials" receipt) && !(std.record.has_field "authority" receipt) then null else Error "migration-overclaim" "receipt contains authority, credentials, or claims",
    ]
in
{ expected = Expected, expected_lock_source = ExpectedLockSource, is_blake3 = IsBlake3, is_git_oid = IsGitOid, required_non_claims = RequiredNonClaims, validate = Validate }
