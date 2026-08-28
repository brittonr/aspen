let Valid = import "../../profiles/local-deterministic.ncl" in
let Contracts = import "../../contracts.ncl" in
(
  Valid
  & {
    unexplained_numeric_thresholds | force = [1000],
  }
) | Contracts.Profile
