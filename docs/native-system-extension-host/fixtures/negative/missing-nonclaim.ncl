let Contracts = import "../../contracts.ncl" in
let Valid = (import "../../profile.ncl").local_live_pilot in
(
  Valid
  & {
    non_claims | force = std.array.drop 1 Valid.non_claims,
  }
) | Contracts.NativeHostProfile
