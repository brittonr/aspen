let Base = import "../../profile.ncl" in
let Contracts = import "../../contracts.ncl" in
(
  Base
  & {
    participants
      | force
      = [
        { participant = "node-a", service = "transactional-key-value", is_required = true },
        { participant = "node-a", service = "replicated-log", is_required = true },
      ],
  }
) | Contracts.Profile
