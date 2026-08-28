let Contracts = import "../../contracts.ncl" in
let Valid = (import "../../profile.ncl").live in
(
  Valid
  & {
    bounds.timeout_ms | force = 0,
  }
) | Contracts.ExecutionProfile
