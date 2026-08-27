let Contracts = import "../../contracts.ncl" in
let Valid = (import "../../profile.ncl").local_live_pilot in
(
  Valid
  & {
    bounds.materialized_value_bytes | force = 0,
  }
) | Contracts.NativeHostProfile
