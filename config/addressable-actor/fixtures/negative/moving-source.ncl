let Base = import "../../profile.ncl" in
let Contracts = import "../../contracts.ncl" in
(Base & { reference_source.revision | force = "main" }) | Contracts.Profile
