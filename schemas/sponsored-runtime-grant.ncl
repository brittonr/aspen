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

let NonNegativeNumber = std.contract.from_validator (fun value =>
  if std.is_number value && value >= 0 then
    'Ok
  else
    'Error { message = "number must be non-negative" }
) in

let SponsoredPrincipalRole = enum_string "SponsoredPrincipalRole" ["sponsor", "beneficiary", "provider", "workload", "service", "organization", "user", "node", "plugin"] in

let SponsoredPrincipalRef = {
  principal_id | String,
  role | SponsoredPrincipalRole,
  proof_ref | String | optional,
} in

let RedactedValue = enum_string "RedactedValue" ["redacted"] in

let SponsoredSettlementReference = {
  method_tag | String,
  opaque_ref | RedactedValue,
} in

let SponsoredResourceLimits = {
  cpu_millis | NonNegativeNumber,
  memory_bytes | NonNegativeNumber,
  storage_bytes_ms | NonNegativeNumber,
  network_bytes | NonNegativeNumber,
  wall_time_ms | NonNegativeNumber,
  max_concurrent | NonNegativeNumber,
} in

let RuntimeHostKind = enum_string "RuntimeHostKind" ["native-built-in", "native-process", "wasm", "hyperlight", "oci-container"] in

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
  valid_from_ms | NonNegativeNumber,
  valid_until_ms | NonNegativeNumber,
  revocation | SponsoredRevocationRef,
  settlement | SponsoredSettlementReference,
  policy_tags | Array String,
} in

let SponsoredPrincipalRoleContract = SponsoredPrincipalRole in
let SponsoredPrincipalRefContract = SponsoredPrincipalRef in
let RedactedValueContract = RedactedValue in
let SponsoredSettlementReferenceContract = SponsoredSettlementReference in
let SponsoredResourceLimitsContract = SponsoredResourceLimits in
let RuntimeHostKindContract = RuntimeHostKind in
let SponsoredGrantScopeContract = SponsoredGrantScope in
let SponsoredRevocationRefContract = SponsoredRevocationRef in
let SponsoredRuntimeGrantContract = SponsoredRuntimeGrant in

{
  SponsoredPrincipalRole = SponsoredPrincipalRoleContract,
  SponsoredPrincipalRef = SponsoredPrincipalRefContract,
  RedactedValue = RedactedValueContract,
  SponsoredSettlementReference = SponsoredSettlementReferenceContract,
  SponsoredResourceLimits = SponsoredResourceLimitsContract,
  RuntimeHostKind = RuntimeHostKindContract,
  SponsoredGrantScope = SponsoredGrantScopeContract,
  SponsoredRevocationRef = SponsoredRevocationRefContract,
  SponsoredRuntimeGrant = SponsoredRuntimeGrantContract,
}
