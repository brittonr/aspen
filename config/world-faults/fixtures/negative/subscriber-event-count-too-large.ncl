let Valid = import "../../profiles/local-deterministic.ncl" in
let Contracts = import "../../contracts.ncl" in
(
  Valid
  & {
    subscriber.event_count | force = 65,
  }
) | Contracts.Profile
