let Base = import "../../ledger.ncl" in
let Contracts = import "../../contracts.ncl" in
(Base & { claims_global_root_identity | force = true }) | Contracts.Profile
