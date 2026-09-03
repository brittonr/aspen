let Base = import "../../profile.ncl" in
let Contracts = import "../../contracts.ncl" in
let InvalidRules = std.array.replace_at 4 { class = "processes", disposition = "durable" } Base.survival.rules in
(Base & { survival.rules | force = InvalidRules, claims_runtime_survival | force = true }) | Contracts.Profile
