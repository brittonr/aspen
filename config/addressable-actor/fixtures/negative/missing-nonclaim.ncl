let Base = import "../../profile.ncl" in
let Contracts = import "../../contracts.ncl" in
let Weakened = std.array.drop 1 Base.non_claims in
(Base & { non_claims | force = Weakened }) | Contracts.Profile
