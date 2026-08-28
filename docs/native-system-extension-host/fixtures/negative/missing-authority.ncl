let Contracts = import "../../contracts.ncl" in
let Valid = (import "../../profile.ncl").local_live_pilot in
(
  Valid
  & {
    authorities | force = std.array.drop 1 Valid.authorities,
  }
) | Contracts.NativeHostProfile
