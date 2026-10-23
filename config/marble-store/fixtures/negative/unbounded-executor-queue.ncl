let Base = import "../../selection.ncl" in
let Contracts = import "../../contracts.ncl" in
(Base & { marble.executor_queue_depth | force = 4096 }) | Contracts.Selection
