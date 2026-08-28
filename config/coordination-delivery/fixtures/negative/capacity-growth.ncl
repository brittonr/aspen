let Base = import "../../profile.ncl" in
let Contracts = import "../../contracts.ncl" in
let ExpandedCapacity = 16 in
(Base & { policy.ready_capacity | force = ExpandedCapacity }) | Contracts.Profile
