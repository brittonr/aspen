let Valid = import "../../profiles/local-deterministic.ncl" in
let Contracts = import "../../contracts.ncl" in
(
  Valid
  & {
    subscriber.phase | force = "after-durable-write",
  }
) | Contracts.Profile
