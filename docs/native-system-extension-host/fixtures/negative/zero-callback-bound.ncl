let Contracts = import "../../contracts.ncl" in
let Valid = (import "../../profile.ncl").local_live_pilot in
(
  Valid
  & {
    bounds.callback_output_bytes | force = 0,
  }
) | Contracts.NativeHostProfile
