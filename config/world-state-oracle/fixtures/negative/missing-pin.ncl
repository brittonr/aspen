let Base = import "../../source.ncl" in
let Contracts = import "../../source-contracts.ncl" in
(Base & { source_revision | force = "" }) | Contracts.Source
