let Base = import "../../profile.ncl" in
let Contracts = import "../../contracts.ncl" in
(
  Base
  & { novelty_fields | force = ["service", "transition-class", "logical-position", "state", "debug-text"] }
) | Contracts.Profile
