let Valid = import "../../profiles/local-deterministic.ncl" in
let Contracts = import "../../contracts.ncl" in
(
  Valid
  & {
    subscriber.stream_epoch | force = 0,
  }
) | Contracts.Profile
