let Contracts = import "../../contracts.ncl" in
let Valid = (import "../../profile.ncl").live in
(
  Valid
  & {
    platform | force = "direct-child-only",
  }
) | Contracts.ExecutionProfile
