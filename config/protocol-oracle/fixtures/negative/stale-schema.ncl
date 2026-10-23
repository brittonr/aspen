let Base = import "../../profile.ncl" in
let Contracts = import "../../contracts.ncl" in
(Base & { schema | force = "molten.protocol-oracle-profile.v0" }) | Contracts.Profile
