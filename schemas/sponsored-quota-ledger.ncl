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

let SponsoredResourceLimits = {
  cpu_millis | Number,
  memory_bytes | Number,
  storage_bytes_ms | Number,
  network_bytes | Number,
  wall_time_ms | Number,
  max_concurrent | Number,
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
  active_concurrency | Number,
} in

{
  SponsoredResourceLimits,
  SponsoredQuotaReservation,
  SponsoredQuotaConsumption,
  SponsoredQuotaLedger,
}
