let Base = import "../../proof-obligations.ncl" in
let Contracts = import "../../proof-contracts.ncl" in
(Base & { formal_refinement_complete | force = true }) | Contracts.Proof
