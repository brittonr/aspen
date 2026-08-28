let Valid = import "../../profiles/local-deterministic.ncl" in
let Contracts = import "../../contracts.ncl" in
(
  Valid
  & {
    limits.max_restarts | force = 0,
  }
) | Contracts.Profile
