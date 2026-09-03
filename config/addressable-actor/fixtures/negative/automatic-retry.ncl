let Base = import "../../profile.ncl" in
let Contracts = import "../../contracts.ncl" in
(Base & { automatic_external_retry | force = true }) | Contracts.Profile
