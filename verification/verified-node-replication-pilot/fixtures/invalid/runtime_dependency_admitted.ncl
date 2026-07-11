let contracts = import "../../contracts.ncl" in
let profile = import "../../profile.ncl" in
(profile & { runtime_dependency_status | force = "admitted" }) | contracts.PilotProfile
