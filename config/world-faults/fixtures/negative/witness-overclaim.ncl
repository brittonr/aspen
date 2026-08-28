let Valid = import "../../profiles/local-deterministic.ncl" in
let Contracts = import "../../contracts.ncl" in
(
  Valid
  & {
    claims_independent_witness | force = true,
  }
) | Contracts.Profile
