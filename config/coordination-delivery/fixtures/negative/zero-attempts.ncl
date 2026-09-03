let Base = import "../../profile.ncl" in
let Contracts = import "../../contracts.ncl" in
let ZeroAttempts = 0 in
(Base & { policy.maximum_attempts | force = ZeroAttempts }) | Contracts.Profile
