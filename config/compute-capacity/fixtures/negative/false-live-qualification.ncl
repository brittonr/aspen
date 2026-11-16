let Base = import "../../profile.ncl" in
let Contracts = import "../../contracts.ncl" in
(Base & { live_qualification | force = true }) | Contracts.Profile
