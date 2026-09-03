let Base = import "../../profile.ncl" in
let Contracts = import "../../contracts.ncl" in
(Base & { receipt_grants_authority | force = true }) | Contracts.Profile
