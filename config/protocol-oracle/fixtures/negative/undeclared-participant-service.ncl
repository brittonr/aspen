let Base = import "../../profile.ncl" in
let Contracts = import "../../contracts.ncl" in
(
  Base
  & {
    services
      | force
      = [
        {
          service = "transactional-key-value",
          logical_position = "version",
          safety = ["local", "durability"],
          physical_layout = "semantic-only",
        },
        {
          service = "replicated-log",
          logical_position = "next-offset",
          safety = ["local", "durability"],
          physical_layout = "semantic-only",
        },
      ],
  }
) | Contracts.Profile
