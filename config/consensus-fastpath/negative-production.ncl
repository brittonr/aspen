let Profile = (import "profile.ncl").three_replica in
let Contracts = import "contracts.ncl" in
(Profile & { selection | force = "production" }) | Contracts.Profile
