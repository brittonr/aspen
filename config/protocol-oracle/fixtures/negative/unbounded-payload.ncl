let Base = import "../../profile.ncl" in
let Contracts = import "../../contracts.ncl" in
(Base & { bounds.max_total_bytes | force = 32768 }) | Contracts.Profile
