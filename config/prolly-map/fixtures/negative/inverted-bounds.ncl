let Base = import "../../profile.ncl" in
let Contracts = import "../../contracts.ncl" in
(Base & { target_node_bytes | force = Base.max_node_bytes }) | Contracts.Profile
