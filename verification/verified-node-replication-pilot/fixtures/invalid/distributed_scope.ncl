let contracts = import "../../contracts.ncl" in
let profile = import "../../profile.ncl" in
(profile & { scope | force = "distributed-network-replication-and-consensus" }) | contracts.PilotProfile
