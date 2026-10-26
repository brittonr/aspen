let Valid = import "../../profiles/local-deterministic.ncl" in
let Contracts = import "../../contracts.ncl" in
(
  Valid
  & {
    subscriber.buffer_bound | force = Valid.subscriber.event_count,
  }
) | Contracts.Profile
