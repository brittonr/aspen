let Base = import "../../profile.ncl" in
let Contracts = import "../../contracts.ncl" in
(Base & { boundary_seed_ref | force = "" }) | Contracts.Profile
