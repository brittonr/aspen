let Base = import "../../source.ncl" in
let Contracts = import "../../source-contracts.ncl" in
(Base & { production_enabled | force = true }) | Contracts.Source
