# Auto-generated Nickel contracts. Do not edit manually.
# Contract family: sponsored-quota-ledger
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

let SponsoredResourceLimits = {
  cpu_millis | NonNegativeNumber,
  memory_bytes | NonNegativeNumber,
  storage_bytes_ms | NonNegativeNumber,
  network_bytes | NonNegativeNumber,
  wall_time_ms | NonNegativeNumber,
  max_concurrent | NonNegativeNumber,
} in

let SponsoredQuotaReservation = {
  reservation_id | String,
  grant_id | String,
  workload_id | String,
  reserved | SponsoredResourceLimits,
} in

let SponsoredQuotaConsumption = {
  consumption_id | String,
  reservation_id | String,
  consumed | SponsoredResourceLimits,
  released | SponsoredResourceLimits,
} in

let SponsoredQuotaLedger = {
  grant_id | String,
  total | SponsoredResourceLimits,
  reserved | SponsoredResourceLimits,
  consumed | SponsoredResourceLimits,
  active_concurrency | NonNegativeNumber,
} in

let SponsoredResourceLimitsContract = SponsoredResourceLimits in
let SponsoredQuotaReservationContract = SponsoredQuotaReservation in
let SponsoredQuotaConsumptionContract = SponsoredQuotaConsumption in
let SponsoredQuotaLedgerContract = SponsoredQuotaLedger in

{
  SponsoredResourceLimits = SponsoredResourceLimitsContract,
  SponsoredQuotaReservation = SponsoredQuotaReservationContract,
  SponsoredQuotaConsumption = SponsoredQuotaConsumptionContract,
  SponsoredQuotaLedger = SponsoredQuotaLedgerContract,
}
