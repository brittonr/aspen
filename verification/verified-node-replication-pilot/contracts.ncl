let NonEmptyString = std.contract.from_predicate (fun value =>
  std.is_string value && std.string.length value > 0
) in
let NonEmptyArray = std.contract.from_predicate (fun value =>
  std.is_array value && std.array.length value > 0
) in
let GitRevision = std.contract.from_predicate (fun value =>
  std.is_string value && std.string.is_match "^[0-9a-f]{40}$" value
) in
let Blake3Hex = std.contract.from_predicate (fun value =>
  std.is_string value && std.string.is_match "^[0-9a-f]{64}$" value
) in
let NixSha256 = std.contract.from_predicate (fun value =>
  std.is_string value && std.string.is_match "^sha256-[A-Za-z0-9+/]+={0,2}$" value
) in
let PositiveInteger = std.contract.from_predicate (fun value =>
  std.is_number value && value > 0 && value % 1 == 0
) in
let ExactNumber = fun expected => std.contract.from_predicate (fun value =>
  std.is_number value && value == expected
) in
let TrustedMarkerCount = ExactNumber 33 in
let ExternalBodyCount = ExactNumber 9 in
let AssumeCount = ExactNumber 1 in
let UnsupportedFeatureDiagnosticCount = ExactNumber 7 in
let OneOf = fun allowed => std.contract.from_predicate (fun value =>
  std.is_string value && std.array.elem value allowed
) in
{
  PilotProfile = {
    schema_version | OneOf ["molten-verified-node-replication-pilot/v1"],
    pilot_id | NonEmptyString,
    scope | OneOf ["local-multicore-numa-data-structure-only"],
    upstream | {
      project | OneOf ["verus-lang/verified-node-replication"],
      repository_url | NonEmptyString,
      revision | GitRevision,
      source_hash | NixSha256,
      source_hash_protocol | OneOf ["nix-sha256-unpacked"],
      tree_api_hash | NixSha256,
      license | OneOf ["MIT"],
      sentinels | NonEmptyArray | Array NonEmptyString,
    },
    historical_verus | {
      project | OneOf ["verus-lang/verus"],
      revision | GitRevision,
      source_hash | NixSha256,
      source_hash_protocol | OneOf ["nix-sha256-unpacked"],
      rust_version | OneOf ["1.76.0"],
    },
    current_probe | {
      profile_id | OneOf ["octet-verus-0.2026.04.03.21dfcd2"],
      verifier | OneOf ["verus@0.2026.04.03.21dfcd2"],
      verifier_release_sha256_hex | Blake3Hex,
      verifier_release_nix_sri | NixSha256,
      verifier_binary_blake3 | Blake3Hex,
      solver_blake3 | Blake3Hex,
      rust_version | OneOf ["1.94.0"],
      command_argv | NonEmptyArray | Array NonEmptyString,
      timeout_seconds | PositiveInteger,
      maximum_saved_log_bytes | PositiveInteger,
      unsupported_feature_diagnostic_count | UnsupportedFeatureDiagnosticCount,
      expected_decision | OneOf ["blocked-verifier-internal-error"],
      required_diagnostic_anchors | NonEmptyArray | Array NonEmptyString,
    },
    trusted_boundary | {
      trusted_marker_count | TrustedMarkerCount,
      external_body_count | ExternalBodyCount,
      assume_count | AssumeCount,
      required_locations | NonEmptyArray | Array NonEmptyString,
    },
    runtime_dependency_status | OneOf ["denied"],
    promotion_criteria | NonEmptyArray | Array NonEmptyString,
    claim_scope | OneOf ["compatibility-and-trusted-boundary-pilot-only"],
    non_claims | NonEmptyArray | Array NonEmptyString,
  },
}
