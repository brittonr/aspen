let Base = import "../../ledger.ncl" in
let Contracts = import "../../contracts.ncl" in
(
  Base
  & {
    limits.max_intentional | force = 6,
  }
) | Contracts.Profile
