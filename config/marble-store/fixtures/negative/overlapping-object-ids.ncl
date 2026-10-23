let Base = import "../../selection.ncl" in
let Contracts = import "../../contracts.ncl" in
(Base & { marble.block_ids.start | force = 0 }) | Contracts.Selection
