# Auto-generated Nickel contracts. Do not edit manually.
# Contract family: sponsored-runtime-grant
# Source: crates/aspen-runtime-core/src/lib.rs
# Regenerate with: python3 scripts/generate-typed-nickel-contracts.py --write

let contains = fun values value =>
  std.array.fold_left (fun found item => found || item == value) false values in

let enum_string = fun field allowed =>
  std.contract.from_validator (fun value =>
    if std.is_string value && contains allowed value then
      'Ok
    else
      'Error { message = "%{field} must be one of %{std.serialize 'Json allowed}" }
  ) in

let SponsoredPrincipalRole = enum_string "SponsoredPrincipalRole" ["Sponsor", "Beneficiary", "Provider", "Workload", "Service", "Organization", "User", "Node", "Plugin"] in

let SponsoredPrincipalRef = {
  principal_id | String,
  role | SponsoredPrincipalRole,
  proof_ref | String | optional,
} in

let RedactedValue = enum_string "RedactedValue" ["Redacted"] in

let SponsoredSettlementReference = {
  method_tag | String,
  opaque_ref | RedactedValue,
} in

let SponsoredResourceLimits = {
  cpu_millis | Number,
  memory_bytes | Number,
  storage_bytes_ms | Number,
  network_bytes | Number,
  wall_time_ms | Number,
  max_concurrent | Number,
} in

let RuntimeHostKind = enum_string "RuntimeHostKind" ["NativeBuiltIn", "NativeProcess", "Wasm", "Hyperlight", "OciContainer"] in

let SponsoredGrantScope = {
  workload_principal_ids | Array String,
  service_principal_ids | Array String,
  provider_principal_ids | Array String,
  node_principal_ids | Array String,
  plugin_principal_ids | Array String,
  resource_classes | Array String,
  isolation_modes | Array RuntimeHostKind,
} in

let SponsoredRevocationRef = {
  revocation_id | String,
  revoked | Bool,
} in

let SponsoredRuntimeGrant = {
  grant_id | String,
  sponsor | SponsoredPrincipalRef,
  beneficiary | SponsoredPrincipalRef,
  provider_scope | Array SponsoredPrincipalRef,
  workload_scope | SponsoredGrantScope,
  limits | SponsoredResourceLimits,
  valid_from_ms | Number,
  valid_until_ms | Number,
  revocation | SponsoredRevocationRef,
  settlement | SponsoredSettlementReference,
  policy_tags | Array String,
} in

{
  SponsoredPrincipalRole,
  SponsoredPrincipalRef,
  RedactedValue,
  SponsoredSettlementReference,
  SponsoredResourceLimits,
  RuntimeHostKind,
  SponsoredGrantScope,
  SponsoredRevocationRef,
  SponsoredRuntimeGrant,
}
