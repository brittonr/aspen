let Base = import "../../profile.ncl" in
let Contracts = import "../../contracts.ncl" in
(Base & { debug_payload = "raw" }) | Contracts.Profile
