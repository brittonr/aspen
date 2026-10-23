let Base = import "../../decision.ncl" in
let Contracts = import "../../contracts.ncl" in
(Base & { verdict | force = "replace" }) | Contracts.Decision
