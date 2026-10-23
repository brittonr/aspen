let Base = import "../../profile.ncl" in
let Contracts = import "../../contracts.ncl" in
(
  Base
  & {
    non_claims
      | force
      =
        std.array.filter
          (fun claim => claim != std.array.first Contracts.required_non_claims)
          Contracts.required_non_claims,
  }
) | Contracts.Profile
