let Contracts = import "../../contracts.ncl" in
let Valid = (import "../../profile.ncl").local_live_pilot in
(
  Valid
  & {
    requires_materialized_values | force = false,
  }
) | Contracts.NativeHostProfile
