let Base = import "../../decision.ncl" in
let Contracts = import "../../contracts.ncl" in
(Base & { claim_verb | force = "proves" }) | Contracts.Decision
