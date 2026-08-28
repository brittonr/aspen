let Base = import "../../profile.ncl" in
let Contracts = import "../../contracts.ncl" in
(Base & { inline_payloads_allowed | force = true }) | Contracts.Profile
