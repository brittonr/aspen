# Auto-generated Nickel contracts. Do not edit manually.
# Contract family: sponsored-usage-receipt
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

let RedactedValue = enum_string "RedactedValue" ["redacted"] in

let RuntimeDiagnostic = {
  key | String,
  value | RedactedValue,
} in

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

let SponsoredReceiptOutcome = enum_string "SponsoredReceiptOutcome" ["started", "reserved", "consumed", "completed", "failed", "revocation-denied", "quota-denied", "policy-denied"] in

let SponsoredUsageReceipt = {
  schema | String,
  receipt_id | String,
  execution_id | String,
  workload_principal_id | String,
  service_principal_id | String | optional,
  provider_principal_id | String,
  sponsor_principal_id | String,
  grant_id | String,
  measured | SponsoredResourceLimits,
  started_at_ms | NonNegativeNumber,
  completed_at_ms | NonNegativeNumber | optional,
  outcome | SponsoredReceiptOutcome,
  artifact_refs | Array String,
  isolation_summary | String,
  settlement | SponsoredSettlementReference,
  diagnostics | Array RuntimeDiagnostic,
} in

let SignedSponsoredUsageReceipt = {
  receipt | SponsoredUsageReceipt,
  signer_principal_id | String,
  signature_ref | RedactedValue,
} in

let RedactedValueContract = RedactedValue in
let RuntimeDiagnosticContract = RuntimeDiagnostic in
let SponsoredSettlementReferenceContract = SponsoredSettlementReference in
let SponsoredResourceLimitsContract = SponsoredResourceLimits in
let SponsoredReceiptOutcomeContract = SponsoredReceiptOutcome in
let SponsoredUsageReceiptContract = SponsoredUsageReceipt in
let SignedSponsoredUsageReceiptContract = SignedSponsoredUsageReceipt in

{
  RedactedValue = RedactedValueContract,
  RuntimeDiagnostic = RuntimeDiagnosticContract,
  SponsoredSettlementReference = SponsoredSettlementReferenceContract,
  SponsoredResourceLimits = SponsoredResourceLimitsContract,
  SponsoredReceiptOutcome = SponsoredReceiptOutcomeContract,
  SponsoredUsageReceipt = SponsoredUsageReceiptContract,
  SignedSponsoredUsageReceipt = SignedSponsoredUsageReceiptContract,
}
