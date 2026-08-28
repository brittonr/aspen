let Base = import "../../source.ncl" in
let Contracts = import "../../source-contracts.ncl" in
(Base & { build_inputs | force = ["gnumake"] }) | Contracts.Source
