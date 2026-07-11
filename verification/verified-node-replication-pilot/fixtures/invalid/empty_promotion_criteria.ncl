let contracts = import "../../contracts.ncl" in
let profile = import "../../profile.ncl" in
(profile & { promotion_criteria | force = [] }) | contracts.PilotProfile
