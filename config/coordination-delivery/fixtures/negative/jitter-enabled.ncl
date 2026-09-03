let Base = import "../../profile.ncl" in
let Contracts = import "../../contracts.ncl" in
(Base & { jitter | force = "bounded" }) | Contracts.Profile
