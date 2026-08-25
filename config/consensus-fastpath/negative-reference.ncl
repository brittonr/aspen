let Profile = (import "profile.ncl").three_replica in
let Contracts = import "contracts.ncl" in
(Profile & {
  source.artifact_revision | force = "floating-main",
}) | Contracts.Profile
