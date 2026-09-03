let Base = import "../../profile.ncl" in
let Contracts = import "../../contracts.ncl" in
(Base & { time_domain | force = "wall-clock" }) | Contracts.Profile
