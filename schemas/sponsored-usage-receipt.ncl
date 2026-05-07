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

let RedactedValue = enum_string "RedactedValue" ["Redacted"] in

let RuntimeDiagnostic = {
  key | String,
  value | RedactedValue,
} in

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

let SponsoredReceiptOutcome = enum_string "SponsoredReceiptOutcome" ["Started", "Reserved", "Consumed", "Completed", "Failed", "RevocationDenied", "QuotaDenied", "PolicyDenied"] in

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
  started_at_ms | Number,
  completed_at_ms | Number | optional,
  outcome | SponsoredReceiptOutcome,
  artifact_refs | Array String,
  isolation_summary | String,
  settlement | SponsoredSettlementReference,
  diagnostics | Array RuntimeDiagnostic,
} in

{
  RedactedValue,
  RuntimeDiagnostic,
  SponsoredSettlementReference,
  SponsoredResourceLimits,
  SponsoredReceiptOutcome,
  SponsoredUsageReceipt,
}
