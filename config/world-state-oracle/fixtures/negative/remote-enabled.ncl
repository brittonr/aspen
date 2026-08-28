let Base = import "../../ledger.ncl" in
let Contracts = import "../../contracts.ncl" in
(Base & { remotes_enabled | force = true }) | Contracts.Profile
