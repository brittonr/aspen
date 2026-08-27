let Contracts = import "../../contracts.ncl" in
let Valid = (import "../../profile.ncl").local_live_pilot in
(
  Valid
  & {
    execution_profile | force = "in-process-native",
  }
) | Contracts.NativeHostProfile
