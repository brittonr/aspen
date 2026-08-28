let Base = import "../../profile.ncl" in
let Contracts = import "../../contracts.ncl" in
let Weakened = std.array.drop 1 Base.policy.non_claims in
(
  Base
  & {
    policy.non_claims | force = Weakened,
    manifest.non_claims | force = Weakened,
  }
) | Contracts.Profile
