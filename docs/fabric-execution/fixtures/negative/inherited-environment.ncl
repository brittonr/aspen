let Contracts = import "../../contracts.ncl" in
let Valid = (import "../../profile.ncl").live in
(
  Valid
  & {
    environment_mode | force = "inherit",
  }
) | Contracts.ExecutionProfile
