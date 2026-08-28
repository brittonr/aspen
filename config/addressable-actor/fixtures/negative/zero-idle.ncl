let Base = import "../../profile.ncl" in
let Contracts = import "../../contracts.ncl" in
(Base & { idle_after_ticks | force = 0 }) | Contracts.Profile
