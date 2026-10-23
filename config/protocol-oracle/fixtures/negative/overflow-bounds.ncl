let Base = import "../../profile.ncl" in
let Contracts = import "../../contracts.ncl" in
(Base & { bounds.max_records | force = 65537 }) | Contracts.Profile
