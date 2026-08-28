let Base = import "../../profile.ncl" in
let Contracts = import "../../contracts.ncl" in
let WeakenedMaximumNodeBytes = 8192 in
(Base & { max_node_bytes | force = WeakenedMaximumNodeBytes }) | Contracts.Profile
